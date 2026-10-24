fn main() -> gtk::glib::ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--generate-thumbnails") {
        return gtk::glib::ExitCode::from(man_gui::oobe::generate_thumbnails() as u8);
    }
    man_gui::oobe::main()
}
