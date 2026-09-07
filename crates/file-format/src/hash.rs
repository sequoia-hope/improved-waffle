//! Content hashing for the v4 `sources` table
//! (`specs/waffle_v4_document_model.md` §2.3).
//!
//! Phase-1 algorithm: the **git blob id** — SHA-1 over `"blob <len>\0" +
//! bytes` — so a hash computed offline matches what GitHub, GitLab and Gitea
//! report for the same file, and a cached `embed` can be verified against a
//! pinned commit with no network at all. The value is self-describing
//! (`git-blob-sha1:<40 hex>`); an unknown prefix is "no hash", never a
//! mismatch (§4 invariant 9).

use sha1::{Digest, Sha1};

/// Algorithm prefix for git blob ids.
pub const GIT_BLOB_SHA1_PREFIX: &str = "git-blob-sha1:";

/// `git-blob-sha1:<hex>` of `bytes` — identical to `git hash-object` on the
/// same content.
pub fn git_blob_sha1(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(GIT_BLOB_SHA1_PREFIX.len() + 40);
    out.push_str(GIT_BLOB_SHA1_PREFIX);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Outcome of checking bytes against a recorded `content_hash`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashCheck {
    /// Recorded hash present, known algorithm, equal.
    Match,
    /// Recorded hash present, known algorithm, different.
    Mismatch { recorded: String, actual: String },
    /// No recorded hash, or an algorithm this reader does not know.
    Unverifiable,
}

/// Check `bytes` against an optional recorded hash string.
pub fn check_content_hash(recorded: Option<&str>, bytes: &[u8]) -> HashCheck {
    match recorded {
        Some(r) if r.starts_with(GIT_BLOB_SHA1_PREFIX) => {
            let actual = git_blob_sha1(bytes);
            if actual.eq_ignore_ascii_case(r) {
                HashCheck::Match
            } else {
                HashCheck::Mismatch {
                    recorded: r.to_string(),
                    actual,
                }
            }
        }
        _ => HashCheck::Unverifiable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known vectors: `git hash-object` of the empty file and of "hello\n".
    #[test]
    fn git_blob_sha1_matches_git_hash_object() {
        assert_eq!(
            git_blob_sha1(b""),
            "git-blob-sha1:e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
        assert_eq!(
            git_blob_sha1(b"hello\n"),
            "git-blob-sha1:ce013625030ba8dba906f756967f9e9ca394464a"
        );
    }

    #[test]
    fn hash_check_semantics() {
        let h = git_blob_sha1(b"hello\n");
        assert_eq!(check_content_hash(Some(&h), b"hello\n"), HashCheck::Match);
        assert!(matches!(
            check_content_hash(Some(&h), b"hello"),
            HashCheck::Mismatch { .. }
        ));
        assert_eq!(check_content_hash(None, b"x"), HashCheck::Unverifiable);
        assert_eq!(
            check_content_hash(Some("sha256:deadbeef"), b"x"),
            HashCheck::Unverifiable,
            "an unknown algorithm is never a mismatch"
        );
        // Case-insensitive on the hex part.
        assert_eq!(
            check_content_hash(
                Some(&h.to_uppercase().replace("GIT-BLOB-SHA1", "git-blob-sha1")),
                b"hello\n"
            ),
            HashCheck::Match
        );
    }
}
