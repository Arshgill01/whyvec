# Compiler product re-foundation audit

## Decision under review

WhyVec began as one counterfactual experiment for one missed LLVM loop-vectorization pattern. The mechanism is real, but that product boundary is too narrow. The durable product is a **causal debugger for compiler decisions**.

It answers two concrete questions that ordinary diagnostics do not answer reliably:

1. **Build causality:** which independently testable changes in a working-tree patch are sufficient to produce a selected compiler failure, and which reported diagnostics disappear with that cause?
2. **Optimization causality:** which isolated, typed compiler assumption is sufficient to change a selected optimization decision under the recorded frontend, optimizer, target, flags, and pipeline?

These are not two unrelated tools. They share the same experimental primitive:

```text
record baseline
→ declare finite interventions
→ execute isolated variants
→ match the same compiler observation
→ find sufficient intervention sets
→ report evidence without upgrading it into truth
```

The working repository name remains WhyVec while this architecture is validated. Product naming must not drive the technical design.

## Non-negotiable product contract

Every supported query type must have:

- a precise subject: diagnostic, optimization decision, or compiler divergence;
- a passing or observed baseline;
- a typed and finite intervention space;
- a three-valued oracle: observed, not observed, or unresolved;
- stable observation identity across variants;
- isolation evidence for each intervention;
- an explicit minimality statement;
- a source-level action, obligation, or refusal;
- a reproduction bundle.

A language logo is not support. A frontend is supported only when its adapter can capture the real invocation, identify observations, apply a typed intervention, rerun the compiler, and explain the limits of the resulting source action.

## Complete defect register

### Load-bearing product defects

| ID | Defect | Why it matters | Technical response | Kill condition |
| --- | --- | --- | --- | --- |
| P-001 | One C aliasing loop is a research fixture, not a product | The audience and visible impact are too narrow | Make vectorization one optimization experiment pack. Add build-causality queries that work across Clang/C++, rustc/Cargo, and TypeScript compiler adapters | Kill the broader product if two language adapters cannot share the experiment engine without collapsing into string parsing |
| P-002 | “Compiler remarks are vague” is not universally true | LLVM can name blocking instructions and emit multiple analysis remarks | Capture the complete structured remark first. Classify whether the tested relationship was already explicit. Counterfactual value must be measured against the actual remark, not an invented weak baseline | Decline `observation.already_explained` when the report already contains the same condition and actionable source relationship |
| P-003 | The pointer-loaded-bound example can look manufactured or obvious | A senior engineer may suspect aliasing immediately | Keep it as a semantic safety fixture, not the only impact case. Require a real-repository case where the relation crosses an ABI, template, macro, generated binding, or non-local caller boundary | Do not use the fixture as the sole evidence of impact |
| P-004 | WhyVec currently requires the user to select a loop | Most developers know a workload is slow, not which compiler decision matters | Add hot-region ingestion from profiles or explicit benchmark symbols, then rank missed decisions by executed hotness and estimated relevance | Do not claim automatic discovery until profile-to-source identity is reliable |
| P-005 | A changed optimization decision is not a speedup | Vectorization can lose after guard and code-size costs | Make benchmark and code-size validation separate observations. Never graduate an optimization repair solely from a compiler remark | Refuse the repair when measured benefit is absent, unstable, or workload-inappropriate |
| P-006 | The current name and CLI encode vectorization as the whole product | It makes every expansion feel bolted on | Introduce query and experiment-pack identities in the domain model before renaming the binary | Rename only after two query types are executable |

### Compiler and experiment defects

| ID | Defect | Why it matters | Technical response | Kill condition |
| --- | --- | --- | --- | --- |
| C-001 | Replaying `default<O3>` is not necessarily the frontend's exact pipeline | A counterfactual may succeed in a surrogate pipeline but not the real build | Record `pipeline_fidelity` as `exact`, `equivalent_confirmed`, or `surrogate`. A successful finding requires exact or independently confirmed equivalence | A surrogate result cannot authorize a source repair |
| C-002 | Rust and Clang may use different LLVM major versions and frontend passes | Common LLVM IR does not imply an identical experiment | Pair every frontend profile with its matching optimizer. Keep frontend lowering and pipeline provenance in the experiment identity | Decline cross-version IR or optimizer use |
| C-003 | Parameter-level LLVM `noalias` is broader than a selected byte-range relation | Translating it directly can create undefined behavior | Separate `compiler_intervention` from `source_obligation`. Add scoped alias experiments where LLVM can represent them, and family-specific access analysis before deriving a range guard | Never emit `restrict`, references, `llvm.assume`, or alias metadata from the counterfactual alone |
| C-004 | The apparent change may be caused by an earlier pass | “The vectorizer needed X” may be false | Retain before/after IR at the measured boundary and pass instrumentation. Report the final changed outcome and the earliest observed divergence separately | Use `inconclusive.direct_cause` when the pass boundary cannot be localized |
| C-005 | Loop identity can drift after inlining, unrolling, cloning, or monomorphization | Variant comparisons can match different code | Use source digest, frontend item identity, debug scope, inline chain, structural IR fingerprint, and parent region. Require family-specific identity thresholds | Decline ambiguous matches; never select the nearest source line |
| C-006 | Optimization records are incomplete and compiler-specific | Absence of a remark is not evidence of absence | Combine structured records with verified IR/codegen observations. Give each adapter an observation completeness declaration | Decline when the requested outcome cannot be observed reliably |
| C-007 | Counterfactual search can explode combinatorially | Minimality claims become expensive or dishonest | Search typed finite families, memoize variants, use deterministic subset strategies, and expose evaluated/skipped sets | Do not call a result minimal unless all required smaller sets were evaluated |
| C-008 | More information can alter unrelated transformations | One attribute can have function-wide effects | Structural-diff the intervention boundary, retain downstream transformation deltas, and state intervention scope explicitly | Decline when the declared intervention itself cannot be isolated |
| C-009 | Compiler and optimizer crashes, timeouts, and nondeterminism are normal outcomes | Treating them as ordinary misses corrupts the search | Use a three-valued oracle plus typed tool failures; repeat successful and unstable candidates | Never coerce unresolved into observed or not observed |
| C-010 | Build commands may execute wrappers, generators, plugins, and scripts | Repository analysis can execute hostile code | Separate trusted compiler adapters from explicitly authorized build adapters; sandbox, deny network by default, fingerprint tools, and bound output/process trees | Refuse opaque execution rather than silently running it |

### Language-scope defects

| ID | Defect | Why it matters | Technical response | Kill condition |
| --- | --- | --- | --- | --- |
| L-001 | “C and C++” is documented but only C is exercised | The claim is presently unearned | Add first-class C++ fixtures including templates, inline functions, and macro-originated loops with mapping declines | Do not list C++ support until positive and refusal paths pass |
| L-002 | Rust can emit LLVM IR, but source contracts differ radically from C | An LLVM experiment does not tell whether a Rust raw pointer, reference, slice, or FFI boundary can enforce the condition | Add a rustc/Cargo adapter and a Rust obligation mapper. Treat safe references, raw pointers, and FFI items as distinct contract families | Do not translate LLVM argument attributes into Rust source syntax generically |
| L-003 | TypeScript does not share LLVM optimization decisions | A fake common IR abstraction would become formatting glue | Support TypeScript first through build-causality queries using compiler diagnostic identities, not through LLVM optimization packs | Do not claim one universal semantic model for incompatible compiler questions |
| L-004 | GCC exposes different optimization records and cost models | Normalizing text would erase useful semantics | Implement GCC as its own observation adapter and use cross-compiler divergence as evidence, not as a replacement for LLVM interventions | Do not claim an LLVM finding automatically applies to GCC |
| L-005 | Cargo, CMake, Ninja, Bazel, and TypeScript builds select compilation units differently | A file path is not enough to reproduce the real compiler state | Each build adapter must resolve one exact compilation unit, environment, target, dependency state, and command digest | Decline ambiguous or unreplayable build selection |

### Build-causality defects

| ID | Defect | Why it matters | Technical response | Kill condition |
| --- | --- | --- | --- | --- |
| B-001 | Delta debugging is established research, not novel by itself | `ddmin` wrapped around a build command is not enough | The product unit is a diagnostic-causality graph: language-aware patch atoms, stable target-diagnostic identity, minimal sufficient edit sets, and measured cascade suppression | Kill this query type if it cannot outperform `git bisect`/manual hunk removal on agent-sized working-tree patches |
| B-002 | Patch atoms are not independent | Removing one hunk can make another fail to apply or parse | Use syntax- and symbol-aware grouping where adapters provide it; classify invalid subsets as unresolved; merge dependency clusters discovered during search | Never interpret an unresolved subset as disproving causality |
| B-003 | Compiler diagnostics change location and wording across variants | Exact string matching loses the target | Fingerprint compiler code, phase, normalized message arguments, primary symbol, source identity, and related spans; permit adapter-specific matching confidence | Decline target tracking below confidence threshold |
| B-004 | One edit can expose a different earlier error | The compiler's recovery order creates non-monotonic observations | Track a set of diagnostic identities and causal suppression edges; use algorithms valid with unresolved/non-monotonic outcomes | Do not promise a unique root cause when multiple minimal sets exist |
| B-005 | Builds are expensive | Naive subset search is unusable on real repositories | Cache by source tree, patch subset, toolchain, dependency, and command digest; compile affected units; parallelize only isolated variants | Surface the search bound and partial result when exhaustive search is not completed |
| B-006 | Generated files and formatting can make textual hunks meaningless | The reported cause may be an artifact rather than an authored change | Trace generated outputs to inputs where possible; otherwise group generated changes and label provenance | Decline source-level attribution for untraceable generated artifacts |

### Source-action and correctness defects

| ID | Defect | Why it matters | Technical response | Kill condition |
| --- | --- | --- | --- | --- |
| S-001 | A tested assumption is not evidence that real callers satisfy it | The easiest patch can be unsound | Require repository contract discovery or runtime enforcement after every optimization finding | Counterfactual output alone never grants patch authority |
| S-002 | Runtime pointer-range guards can introduce undefined or target-invalid arithmetic | A “safe fallback” can itself be unsafe | Use target-specific checked integer-address models only where supported; preserve null, zero-trip, overflow, provenance, and address-space behavior | Refuse runtime enforcement when the target model is incomplete |
| S-003 | Caching a bound can change observable overlap, volatile, signal, or concurrent behavior | The optimized path can alter semantics before the guard | Capture only values the original necessarily evaluates; guard before strengthened assumptions; keep the original fallback structurally intact | Decline volatile, atomic, concurrent, reentrant, or signal-visible cases unless modeled explicitly |
| S-004 | Tests do not prove semantic equivalence | Strong language would be misleading | Report covered executions, sanitizer scope, and any formal proof separately | Never use “proved equivalent” for differential tests |
| S-005 | Global annotations affect every caller and future caller | A local speedup can corrupt a public ABI contract | Audit linkage, FFI, indirect calls, declarations, and public documentation; prefer guarded local versioning when closure is unknown | Refuse global strengthening with unknown callers |

### Agent and product-experience defects

| ID | Defect | Why it matters | Technical response | Kill condition |
| --- | --- | --- | --- | --- |
| A-001 | GPT/Codex can appear bolted onto a deterministic compiler tool | Judges and users can remove it without losing the core demo | Give the model the non-deterministic repository task: resolve contracts across callers, compare legal repair strategies, create a patch, and generate adversarial validation. The engine remains sole authority for compiler facts | If the model merely paraphrases the report or writes a predetermined guard, remove that sequence from the claim |
| A-002 | Live model behavior is nondeterministic | A demonstration or validation can drift | Use schema-constrained tool calls, deterministic compiler gates, retained traces, retry policy, and replayable accepted sessions | No source change is accepted without deterministic postconditions |
| A-003 | Compiler jargon hides the visible transformation | General developers may not understand `noalias`, VF, or pass remarks | Primary UI shows question, tested intervention, affected behavior, safe action, and measured result. IR and pass detail stay queryable | The main result must be understandable without reading LLVM IR |
| A-004 | The current repository is mostly contracts and scaffolding | Documentation can create false confidence | Reopen foundation status. Every phase now requires executable evidence, not document presence | No phase is complete solely because schemas and prose exist |
| A-005 | Existing LSP, rustc JSON, LLVM remarks, agent-LSP bridges, and generative compilation cover adjacent territory | “Structured diagnostics for agents” is not novel | Keep differentiation on executed counterfactuals, stable observation identity, minimal sufficient interventions, and source obligations | Kill any feature whose only transformation is prose or JSON normalization |

## Executable product architecture

```text
repository + selected compiler question
                  │
                  ▼
       frontend/build adapter
  (Clang, Cargo/rustc, TypeScript)
                  │
                  ▼
        observation baseline
 (diagnostic, optimization, divergence)
                  │
                  ▼
       typed intervention provider
 (patch atoms or compiler assumption pack)
                  │
                  ▼
       counterfactual experiment engine
  isolation · cache · 3-valued oracle · search
                  │
                  ▼
          causal evidence graph
 interventions → observations → suppressed cascades
                  │
                  ▼
 source obligation / repository repair / refusal
```

### Query: build causality

```console
whyvec explain-build --base HEAD --diagnostic rustc:E0277 -- cargo check -p api
```

The result identifies every minimal sufficient edit set found for the selected diagnostic and records which other diagnostics disappear with each set. It does not claim the edit is semantically wrong; it establishes that the compiler outcome depends on that edit set under the recorded build.

### Query: optimization causality

```console
whyvec explain-opt src/kernel.c:5 --function add_vectors_ \
  --parameter output:0 --parameter input:1 --parameter count:2 \
  --transformer /path/to/whyvec-llvm-transform \
  --identity-tool /path/to/whyvec-llvm-loop-identity
```

The result records the baseline optimizer decision, isolated compiler interventions, the smallest successful set found, pipeline fidelity, and a language-specific candidate obligation or refusal.

## Initial adapter matrix

| Adapter | Build causality | Optimization causality | Source-action layer |
| --- | --- | --- | --- |
| Clang C/C++ | Structured/SARIF diagnostic identity plus compilation database resolution | LLVM remark and IR observation packs | C/C++ AST, linkage, ABI, alias, and guard rules |
| Cargo/rustc | rustc JSON diagnostic identity plus Cargo unit resolution | Matching rustc/LLVM profile where pipeline fidelity is established | safe reference, slice, raw-pointer, unsafe, and FFI contract families |
| TypeScript | Compiler API diagnostics and program graph | Not an LLVM target; no fabricated optimization support | symbol, declaration, module, and configuration edits |
| GCC C/C++ | GCC JSON diagnostic identity | GCC optimization records and divergence observations | GCC-specific attributes and C/C++ source contracts |

## Order of technical resolution

1. Generalize the domain model from loop outcomes to compiler observations, query types, adapters, interventions, and three-valued experiment results.
2. Replace the C-only fixture manifest with per-case frontend and toolchain profiles.
3. Retain executable Clang and rustc/LLVM counterfactual evidence, including pipeline-fidelity labels.
4. Implement the isolated experiment runner and immutable artifact model.
5. Implement Clang baseline and LLVM alias-trip-count pack end to end.
6. Implement build-causality search with Clang and rustc diagnostic identities.
7. Add C++ template/macro mapping cases and Rust raw-pointer/FFI contract cases.
8. Implement source obligations, guarded enforcement, and adversarial behavior checks.
9. Add TypeScript build-causality adapter and GCC observation adapter.
10. Integrate GPT-5.6/Codex only after deterministic report and validation contracts are executable.

This order is based on dependency integrity, not a reduced product promise. The product boundary remains the full causal compiler debugger.
