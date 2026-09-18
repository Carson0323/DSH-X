fn main() {
    println!("cargo:rerun-if-changed=../assets/dsh.ico");
    #[cfg(windows)]
    {
        let icon = std::path::Path::new("../assets/dsh.ico");
        if icon.exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("../assets/dsh.ico");
            res.set("ProductName", "DSH-X");
            res.set("FileDescription", "DSH-X");
            if let Err(error) = res.compile() {
                println!("cargo:warning=embed icon failed: {error}");
            }
        }
    }
}
