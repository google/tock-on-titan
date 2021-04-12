Base source:
https://github.com/rust-lang/libc/tree/3b185efb5ab36d25f4c1f87ce74ccce42ddd4743

Modifications:

1. Added this file
2. rm -rf .cirrus.yml ci/ CONTRIBUTING.md .github .gitignore libc-test/ \
   README.md rustfmt.toml tests triagebot.toml
3. Opened Cargo.toml, removed [Workspace] section and all references to
   rustc-std-workspace-core.
4. rm -r src/{fuchsia/,hermit/,psp.rs,vxworks/,windows/,wasi.rs,sgx.rs,switch.rs}
5. rm -r src/unix/{bsd,haiku,hermit,newlib,redox,solarish}
6. Added a .cargo-checksum.json containing just {"files":{}}
7. Opened src/lib.rs. In the last `cfg_if!` block, removed all references to
   OSes other than "unix".
