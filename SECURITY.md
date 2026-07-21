# Security policy

WhyVec analyzes repositories and may invoke build tooling. A repository can therefore supply hostile source, paths, response files, wrappers, compiler plugins, environment references, and build commands.

## Reporting a vulnerability

Report security issues privately through GitHub's private vulnerability reporting for this repository. Do not publish exploit details in a public issue before a fix and coordinated disclosure are ready.

Include:

- affected version or commit;
- operating system and toolchain;
- minimal reproduction;
- expected and observed behavior;
- whether source mutation, command execution, secret exposure, path escape, or artifact poisoning occurred.

## Security boundaries

The deterministic engine must:

- parse command arguments without shell evaluation;
- reject unsupported wrappers and compiler plugins by default;
- isolate shadow builds from the source tree;
- bound CPU, memory, process count, output size, and execution duration through the execution policy;
- prevent path traversal and symlink escape;
- sanitize logs and artifact names;
- avoid inheriting unrelated credentials or network configuration;
- cryptographically digest inputs and outputs used as evidence.

See [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) for the detailed trust model and required mitigations.
