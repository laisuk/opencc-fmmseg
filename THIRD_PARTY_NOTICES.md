# Third-Party Notices

This project is licensed under **MIT**. It bundles or depends on third-party components that are licensed under
different terms. The following notices are provided for attribution and license compliance.

---

## 1) Open Chinese Convert (OpenCC) — Dictionaries & Configs

**Component:** OpenCC lexicon files, including Seal-script conversion dictionaries and configs  
**Upstream:** Open Chinese Convert (OpenCC)  
**License:** **Apache License 2.0**  
**Files in this distribution:** `dicts/` (and any derived packs generated from these dictionaries)  
**License files included:**

- `dicts/LICENSE` (Apache-2.0, copied from upstream)

**Attribution:**  
Copyright © OpenCC contributors

**Modifications (if any):**

- We may repackage, filter, merge, or precompile dictionary content (e.g., CBOR/Zstd/“dict packs”) for performance.
- This distribution includes OpenCC Seal-script conversion data where available; derived dictionary packs may contain
  these mappings.
- We do **not** change the license on these files; they remain **Apache-2.0**.
- A summary of our changes (if applicable) is recorded in `dicts/CHANGES.md`.

**Notes on usage:**

- The OpenCC lexicon, including its Seal-script mappings, is treated as the canonical linguistic source for the
  corresponding conversion data.
- Any artifacts derived directly from the OpenCC lexicon (e.g., compiled packs) are considered derivative and are
  distributed together with the Apache-2.0 license and notices.

---

## 2) Project License Clarification

- **Our source code:** MIT License (see `LICENSE` in the repository root).
- **Bundled OpenCC lexicon (and derivatives):** Apache License 2.0 (see `dicts/LICENSE`, and `dicts/NOTICE` if present).
- These licenses apply to their respective components: MIT to this project's source code, and Apache-2.0 to the bundled
  OpenCC lexicon and its derivatives.

---

## 3) How to Report Attribution Issues

If you believe any attribution or license terms are missing or incorrect, please open an issue with details so we can
correct it promptly.

---

*End of Third-Party Notices.*
