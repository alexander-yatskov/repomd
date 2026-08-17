# Release checklist

The crates.io package is `repomd-cli`. The binary is `repomd`.

1. Add a crates.io API token as the GitHub Actions secret `CARGO_REGISTRY_TOKEN`.
2. Confirm that `Cargo.toml` and `Cargo.lock` contain the release version.
3. Run:

   ```bash
   cargo fmt --check
   cargo clippy --all-targets --locked -- -D warnings
   cargo test --all-targets --locked
   cargo publish --dry-run --locked
   ```

4. Merge the release commit into `main` and confirm that CI passes.
5. Create and push a matching tag:

   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

The tag workflow verifies the version, builds GitHub release binaries, and publishes the crate. A crates.io version cannot be replaced after publication.
