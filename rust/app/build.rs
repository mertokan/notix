// Windows'ta exe'ye ikon ve surum bilgisi gom; diger platformlarda bir sey yapmaz.
fn main() {
    #[cfg(windows)]
    {
        let mut r = winresource::WindowsResource::new();
        r.set_icon("assets/icon.ico").set("ProductName", "Notix").set("FileDescription", "Notix");
        r.compile().expect("winresource");
    }
}
