# Search CLI contract evidence

`just memoria-integration memoria search_json_reports_lines_and_limit` passed on macOS arm64. The CLI returned versioned JSON with a repository-relative path, one-based line number, matching text, and `limited: true` when a second line existed beyond `--limit 1`. `just memoria-cli-smoke` displayed the `search` command.

Source: candidate working tree based on `78e48ff`; committed provenance follows the candidate commit.
