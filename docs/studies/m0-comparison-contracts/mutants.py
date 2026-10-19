# Usage: mutate.py WORKTREE NAME  -- applies one named mutant (exactly one textual replacement) in place.
import sys, pathlib
wt, name = sys.argv[1], sys.argv[2]
M = {
 # Value-only validity: a restored value corroborates its origin.
 "value_only": ("crates/warp-core/src/echo_operation.rs",
   "    retained\n        && untouched(invocation.node)\n",
   "    retained\n        && (true || untouched(invocation.node))\n"),
 # Strict whole frontier: the scoped stale-basis exception never applies.
 "no_scoped_exception": ("crates/warp-core/src/echo_operation.rs",
   "            && scoped_action_basis_holds_v1(",
   "            && false && scoped_action_basis_holds_v1("),
 # Staging returns its duplicate before checking occurrence material.
 "staging_occurrence_late": ("crates/warp-core/src/coordinator.rs",
   "        self.check_continuum_occurrence_binding(head_key, &envelope)?;\n\n        let ticketed_ingress_id",
   "        let ticketed_ingress_id"),
 # Submission allocates before returning a known witnessed submission's duplicate.
 "submit_allocates_before_dedupe": ("crates/warp-core/src/coordinator.rs",
   "        if let Some(record) = self\n            .submission_by_target\n            .get(&(head_key, ingress_id))\n            .and_then(|submission_id| self.witnessed_submissions.get(submission_id))\n        {\n            return Ok(IntentSubmissionDisposition::Duplicate",
   "        self.next_submission_generation.checked_increment().ok_or(RuntimeError::IntentSubmissionGenerationOverflow)?;\n        if let Some(record) = self\n            .submission_by_target\n            .get(&(head_key, ingress_id))\n            .and_then(|submission_id| self.witnessed_submissions.get(submission_id))\n        {\n            return Ok(IntentSubmissionDisposition::Duplicate"),
 # The recorder allocates before returning an existing record (reached by staging).
 "allocate_before_dedupe": ("crates/warp-core/src/coordinator.rs",
   "    ) -> Result<IntentSubmissionRecord, RuntimeError> {\n        if let Some(record) = self",
   "    ) -> Result<IntentSubmissionRecord, RuntimeError> {\n        self.next_submission_generation.checked_increment().ok_or(RuntimeError::IntentSubmissionGenerationOverflow)?;\n        if let Some(record) = self"),
}
f, old, new = M[name]; p = pathlib.Path(wt, f); s = p.read_text()
n = s.count(old)
if n != 1: sys.exit(f"mutant {name}: expected 1 match, found {n}")
p.write_text(s.replace(old, new)); print(f"applied {name} to {f}")
