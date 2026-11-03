# Maintaining FactoryKit

`skills/factorykit/` is the single source for Codex and Claude Code. When changing the entry point, edit [SKILL.md](skills/factorykit/SKILL.md) and only the necessary references. Do not copy its content into `.agents/skills/` or `.claude/skills/`.

Keep plugin manifests and marketplaces as thin distribution metadata. Do not duplicate the skill or product-specific instructions in them.

Keep only shared development procedures in this repository. Product-specific specifications, verification commands, Feature Maps, personal information, and credentials belong in the target project.

For a change, follow the target repository's existing design to prevent invalid states and verify observable behavior with an appropriate method. Keep the diff small and explainable. Maximize human understanding: tell reviewers why the change was made, what behavior changed, what was verified, and what remains unverified. Choose techniques according to the language, target, and risk. See the [reliable software engineering reference](docs/references/jane-street-reliable-software.md) for rationale and examples.


<!-- ai-agent-coding-guide:start -->
## Coding guide skills

For code and repository Markdown work, read and apply the matching
`.agents/skills/<skill-name>/SKILL.md` installed in this project. Apply only
skills relevant to the files and task. Invoke the matching `$skill-name` in
Codex when explicit skill selection is needed.

Use both relevant skills when changing code and Markdown together. Before
editing, inspect the existing design, callers, and build configuration. After
editing code, run `./.agents/validate` (or name the language if more than one
code validator is installed) and review decisions that automated checks cannot
judge. For Markdown, follow its checks. For review-only requests, do not edit.
Report unrun checks as unverified. Project-specific instructions take precedence
when they conflict.
<!-- ai-agent-coding-guide:end -->
