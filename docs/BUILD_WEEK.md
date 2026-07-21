# OpenAI Build Week fit

The official [OpenAI Build Week page](https://openai.com/build-week/) says the
challenge is for a project built with Codex. It lists technical implementation,
design/user experience, potential impact, and idea quality as judging criteria,
and says strong submissions demonstrate thoughtful use of GPT‑5.6 and Codex.
It also lists a project description, demo video, code repository, and any other
materials required by the Devpost rules as submission materials.

WhyVec's model role is therefore substantive rather than decorative:

- deterministic compiler tools own the baseline and counterfactual outcomes;
- GPT‑5.6, through an installed Codex skill, inspects repository contracts,
  public headers, callers, FFI uncertainty, and tests;
- the model rejects unsafe `restrict` and unconditional bound caching, authors
  the guarded candidate, executes validation, and returns an evidence ledger;
- a fresh Codex CLI 0.144.3 / `gpt-5.6-sol` run is retained under
  `evidence/codex-live/2026-07-21/` without hidden reasoning or tokens.

No generic chat surface or decorative model summary is used. The model makes
the repository-level decision that compiler evidence alone cannot make, while
WhyVec remains the source of truth for compiler facts.
