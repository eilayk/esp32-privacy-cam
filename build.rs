fn main() {
    embuild::espidf::sysenv::output();

    // `espressif/esp-dl`'s prebuilt fbs_model archive may appear after `-lstdc++`
    // in the final link line. Mark these symbols as undefined up-front so the
    // linker extracts their implementations from libstdc++ when it sees it.
    println!("cargo:rustc-link-arg=-Wl,--undefined=_ZSt11_Hash_bytesPKvjj");
    println!(
        "cargo:rustc-link-arg=-Wl,--undefined=_ZNKSt8__detail20_Prime_rehash_policy14_M_need_rehashEjjj"
    );
}
