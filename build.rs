fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "edit — text editor");
        res.set("ProductName", "edit");
        if let Err(e) = res.compile() {
            eprintln!("Failed to embed Windows icon/resources: {e}");
        }
    }
}
