#[macro_use]
extern crate serde_derive;
extern crate serde;

use mini_rxtx::*;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct MsgType {
    a: u32,
}

#[cfg(feature="std")]
#[test]
fn test_roundtrip_std() {
    let msg_orig = MsgType{
        a: 12345,
    };

    let buf = serialize_msg_owned(&msg_orig).unwrap();
    let msg_actual = deserialize_owned(&buf).unwrap();
    assert_eq!(msg_orig, msg_actual);
}

#[test]
fn test_roundtrip_nostd() {
    let msg_orig = MsgType{
        a: 12345,
    };

    let mut dest = vec![0; 1024];
    let encoded = serialize_msg(&msg_orig,&mut dest).unwrap();
    let buf = encoded.framed_slice();

    let mut decode_buf = [0; 1024];
    let msg_actual = deserialize_owned_borrowed(&buf,&mut decode_buf).unwrap(); // requires cargo feature "std"
    assert_eq!(msg_orig, msg_actual);
}
