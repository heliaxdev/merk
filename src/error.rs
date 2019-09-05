error_chain! {
    foreign_links {
        Sled(sled::Error);
    }
}
