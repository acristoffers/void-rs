# VOID

Void is an encrypted store with a file-system like structure.

Encryption is carried out by Rust's crypto library, using `Blake2b` for key
derivation and `AES-256bit-CGM` for encryption.

# Status

**BREAKING CHANGES**: I think I'm the only one who uses this project today, so I made breaking
changes. If you're not me, then use the git revision `e2e07293e9e1bfee6ea20f71ae69d147b55268e` to
decrypt and the new version to recreate the store. Now I don't expect to do any breaking changes
anymore and future versions will include automatic migration.

The library and cli are considered stable. The GUI is something I have not
decided how I want to develop yet.

This application is not compatible with the previous one, present in this repo
as simply void. As far as I'm aware no one is using it apart from me, so I'm
taking the liberty to silently replace it.
