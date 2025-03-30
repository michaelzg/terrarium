pub mod proto {
    tonic::include_proto!("hello");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("hello_descriptor");
}
