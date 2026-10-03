# Search library contract evidence

`just memoria-verify` passed on macOS arm64. `search_is_literal_ordered_bounded_and_confined` exercised `MemoryRepo::search`: sorted paths and one-based lines, literal case-sensitive matching, explicit limiting, bounded excerpts, invalid limits, and malformed Markdown rejection. `rms verify modules/batch-recovery-boundary/implementation.yaml` passed the public-operation and filesystem-safety proof lanes.

`SearchResult` and `SearchMatch` are read results. The locked snapshot operation creates them inside the driver. Their fields are private to the crate; callers inspect them through accessors. No public constructor can invent a result that the repository did not observe.

Source: candidate working tree based on `78e48ff`; committed provenance follows the candidate commit.
