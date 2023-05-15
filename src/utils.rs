pub fn get_server_name_from_hello_client_message(msg: &[u8]) -> Result<String, &str> {
    let session_id_length = (&msg[43..44])[0] as usize;

    let cipher_suites_length = &msg[44 + session_id_length..46 + session_id_length];
    let cipher_suites_length =
        cipher_suites_length[0] as usize * 256 + cipher_suites_length[1] as usize;

    let compression_method_length_start = 46 + session_id_length + cipher_suites_length;
    let compression_method_length =
        (&msg[compression_method_length_start..compression_method_length_start + 1])[0] as usize;

    let extension_length_start = compression_method_length_start + 1 + compression_method_length;
    let extensions_length = &msg[extension_length_start..extension_length_start + 2];
    let extensions_length = extensions_length[0] as usize * 256 + extensions_length[1] as usize;

    let extensions_start = extension_length_start + 2;
    let extensions = &msg[extensions_start..extensions_start + extensions_length];

    let mut i: usize = 0;
    let mut server_name_result = String::from("");

    while i < extensions_length {
        let extension_type = &extensions[i..i + 2];
        let extension_type = extension_type[0] as usize * 256 + extension_type[1] as usize;
        let length = &extensions[i + 2..i + 4];
        let length = length[0] as usize * 256 + length[1] as usize;
        if extension_type != 0 {
            i = i + 4 + length;
            continue;
        }

        let server_name_length = &extensions[i + 7..i + 9];
        let server_name_length =
            server_name_length[0] as usize * 256 + server_name_length[1] as usize;
        let server_name = &extensions[i + 9..i + 9 + server_name_length];
        server_name_result = String::from_utf8(server_name.to_vec()).unwrap();

        break;
    }

    Ok(server_name_result)
}
