#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { API, DiagnosticCategory } from "typescript/unstable/sync";
import { version } from "typescript";

const configArgument = process.argv[2] ?? "tsconfig.json";
const config = path.resolve(configArgument);
if (!fs.existsSync(config)) {
  process.stderr.write(`missing TypeScript project: ${config}\n`);
  process.exit(2);
}

const api = new API({ cwd: process.cwd() });
try {
  const snapshot = api.updateSnapshot({ openProjects: [config] });
  try {
    const project = snapshot.getProject(config) ?? snapshot.getProjects()[0];
    if (!project) {
      throw new Error(`TypeScript did not load project ${config}`);
    }
    const raw = [
      ...project.program.getConfigFileParsingDiagnostics(),
      ...project.program.getProgramDiagnostics(),
      ...project.program.getGlobalDiagnostics(),
      ...project.program.getSyntacticDiagnostics(),
      ...project.program.getBindDiagnostics(),
      ...project.program.getSemanticDiagnostics(),
    ];
    const seen = new Set();
    const diagnostics = [];
    for (const diagnostic of raw) {
      const key = `${diagnostic.fileName ?? ""}:${diagnostic.pos}:${diagnostic.end}:${diagnostic.code}:${diagnostic.text}`;
      if (seen.has(key)) continue;
      seen.add(key);
      const location = diagnostic.fileName
        ? sourceLocation(diagnostic.fileName, diagnostic.pos, diagnostic.end)
        : null;
      diagnostics.push({
        code: `TS${diagnostic.code}`,
        category: categoryName(diagnostic.category),
        message: diagnostic.text,
        file: diagnostic.fileName ? path.resolve(diagnostic.fileName) : null,
        location,
      });
    }
    diagnostics.sort((left, right) =>
      `${left.file ?? ""}:${left.location?.line ?? 0}:${left.location?.column ?? 0}:${left.code}:${left.message}`.localeCompare(
        `${right.file ?? ""}:${right.location?.line ?? 0}:${right.location?.column ?? 0}:${right.code}:${right.message}`,
      ),
    );
    process.stdout.write(
      `${JSON.stringify({
        schema_version: "1.0.0",
        adapter: "typescript",
        compiler_version: version,
        project: config,
        diagnostics,
      })}\n`,
    );
    process.exitCode = diagnostics.some((item) => item.category === "error") ? 1 : 0;
  } finally {
    snapshot.dispose();
  }
} finally {
  api.close();
}

function categoryName(category) {
  switch (category) {
    case DiagnosticCategory.Error:
      return "error";
    case DiagnosticCategory.Warning:
      return "warning";
    case DiagnosticCategory.Suggestion:
      return "suggestion";
    default:
      return "message";
  }
}

function sourceLocation(file, start, end) {
  const source = fs.readFileSync(file, "utf8");
  const before = source.slice(0, Math.max(0, start));
  const lines = before.split(/\r?\n/);
  return {
    line: lines.length,
    column: lines.at(-1).length + 1,
    length: Math.max(0, end - start),
  };
}
