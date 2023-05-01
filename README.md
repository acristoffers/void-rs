# VOID

Void is an encrypted store with a file-system like structure.

Encryption is carried out by Rust's crypto library, using `Blake2b` for key
derivation and `AES-256bit-CGM` for encryption.

# Status

The library and cli are considered stable. The GUI is something I have not
decided how I want to develop yet.

This application is not compatible with the previous one, present in this repo
as simply void. As far as I'm aware no one is using it apart from me, so I'm
taking the liberty to silently replace it.
