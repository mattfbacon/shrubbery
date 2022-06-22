use super::FromMultipart;

#[derive(FromMultipart)]
struct EmptyStruct {}

#[derive(FromMultipart)]
#[multipart(tag = "tag")]
enum EmptyEnum {}

#[derive(FromMultipart)]
struct BasicStruct {
    x: u64,
    y: u32,
    #[multipart(rename = "other_other_thing")]
    other_thing: String,
}

#[derive(FromMultipart)]
#[multipart(tag = "tag")]
enum BasicEnum {
    First {},
    #[multipart(rename = "xyz")]
    Second {
        x: u64,
        y: u32,
    },
    Third {
        z: String,
    },
}

#[cfg(feature = "file")]
mod file {
    use super::FromMultipart;
    use crate::file::File;

    #[derive(FromMultipart)]
    struct X {
        field1: File,
    }
}
