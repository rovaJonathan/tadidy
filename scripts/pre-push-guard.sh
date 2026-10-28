#!/bin/sh
# pre-push: no direct push to main (AGENTS.md §11), same rule as the global ~/.githooks/pre-push
# that this repo's local hooksPath replaces. Only the very first push that creates main is allowed.
# Reads git's pre-push stdin: <local ref> <local sha> <remote ref> <remote sha>.
ZERO=0000000000000000000000000000000000000000
while read -r _ _ remote_ref remote_sha; do
  case "$remote_ref" in
  refs/heads/main | refs/heads/master)
    if [ "$remote_sha" != "$ZERO" ]; then
      echo "⛔ direct push to ${remote_ref#refs/heads/} refused: open a pull request (gh pr create)" >&2
      exit 1
    fi
    ;;
  esac
done
exit 0
