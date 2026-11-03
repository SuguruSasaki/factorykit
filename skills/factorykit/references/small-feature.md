# Small feature change

Use this procedure when a request adds one bounded user-visible behavior to an existing product. Scale the checks to the affected behavior; do not create planning artifacts for their own sake.

## Before changing code

1. State the behavior from the user's point of view, an observable acceptance condition, and the product surface that exposes it (for example CLI, API, or UI). If the request is ambiguous, inspect the current behavior first; ask only for a decision that cannot be inferred.
2. Read the target project's instructions, existing feature description or map, relevant code and callers, and available verification recipe. Check whether the behavior crosses another surface or persistent data boundary.
3. Select a small implementation slice. State the design reason where there is a real tradeoff, including compatibility or data effects. Keep the project's language and document guidance in its own repository.

## Implement and verify

4. Implement the slice and update any product-owned feature description or verification steps in the same change. Add a focused test when it checks a meaningful contract or regression; avoid a test that merely repeats the implementation.
5. Run the project's relevant static checks and tests. Then exercise the changed behavior through the surface users reach, in an isolated environment when feasible. Compare the actual result and side effects with the acceptance condition through an independent observation where possible. If no verification recipe exists, make the smallest safe direct check and record the missing recipe. A passing CLI check does not verify an API or UI surface.
6. Keep evidence tied to the exact code state: commit ID when available, uncommitted diff or file hashes, check or operation, expected result, actual result, and evidence path or concise output. Avoid secrets and personal data. For shared or production data, confirm scope and authorization before acting.

If the environment cannot run a check, mark it `blocked` or `not-run` with the reason. A check that ran and failed is `failed`. Fix a product or verification problem and rerun the affected check when possible; do not weaken the expected result to obtain a pass.

## Report

Give a compact account of the user-visible change, design reason, and checks by status. Name the surfaces verified and those still unverified. Record any remaining limitation and the next concrete check. Preserve enough evidence for another developer to reproduce the conclusion without collecting unnecessary logs.
