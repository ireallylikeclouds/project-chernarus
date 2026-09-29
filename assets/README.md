# assets/

Distributable, project-owned runtime assets. Every file here must have a record in
[`provenance.toml`](provenance.toml) with class `PROJECT-CREATED` or `LICENSED`.
Reference (Bohemia / DayZ Mod) files never go here, in original or converted form.

Enforced by `cargo test -p chernarus-assets --test repository_guard` and
`cargo run -p chernarus-refscan -- provenance check`. See
[docs/assets/provenance.md](../docs/assets/provenance.md).

No runtime assets exist yet (milestone M0 has no renderer).
