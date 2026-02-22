fn main() {
    let root = "HDiffPatch";
    let hpatch_path = format!("{}/libHDiffPatch/HPatch", root);

    // zstd headers for the C code
    let zstd_path = "vendor/zstd";

    let mut build = cc::Build::new();

    build
        .warnings(false)
        .define("_CRT_SECURE_NO_WARNINGS", "1")
        .define("_CompressPlugin_zstd", "1")
        .define("_IS_USED_MULTITHREAD", "0")
        .include(root)
        .include(&hpatch_path)
        .include(zstd_path);

    // Compile
    build
        .file("hpatch_wrapper.c")
        .file(format!("{}/patch.c", hpatch_path))
        .file(format!("{}/file_for_patch.c", root));

    build.compile("hpatch_lite");
}