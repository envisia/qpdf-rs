use std::collections::HashSet;

use qpdf::*;

fn load_pdf() -> Qpdf {
    Qpdf::read("tests/data/test.pdf").unwrap()
}

#[test]
fn test_qpdf_version() {
    assert_eq!(Qpdf::library_version(), "10.5.0");
    println!("{}", Qpdf::library_version());
}

#[test]
fn test_writer() {
    let qpdf = load_pdf();
    let mut writer = qpdf.writer();
    writer
        .force_pdf_version("1.7")
        .content_normalization(true)
        .preserve_unreferenced_objects(true)
        .object_stream_mode(ObjectStreamMode::Disable)
        .linearize(true)
        .compress_streams(true)
        .stream_data_mode(StreamDataMode::Compress);

    let mem = writer.write_to_memory().unwrap();

    let mem_pdf = Qpdf::read_from_memory(&mem).unwrap();
    assert_eq!(mem_pdf.get_pdf_version(), "1.7");
    assert!(mem_pdf.is_linearized());
}

#[test]
fn test_pdf_from_scratch() {
    let qpdf = Qpdf::empty();

    let font = qpdf
        .parse_object(
            r#"<<
                        /Type /Font
                        /Subtype /Type1
                        /Name /F1
                        /BaseFont /Helvetica
                        /Encoding /StandardEncoding
                      >>"#,
        )
        .unwrap()
        .make_indirect();

    let procset = qpdf.parse_object("[/PDF /Text]").unwrap().make_indirect();
    let contents = qpdf.new_stream(b"BT /F1 15 Tf 72 720 Td (First Page) Tj ET\n");
    let mediabox = qpdf.parse_object("[0 0 612 792]").unwrap();
    let rfont = qpdf.new_dictionary_from([("/Font", font)]);
    let resources = qpdf.new_dictionary_from([("/ProcSet", procset), ("/Font", rfont.inner)]);
    let page = qpdf
        .new_dictionary_from([
            ("/Type", qpdf.new_name("/Page")),
            ("/MediaBox", mediabox),
            ("/Contents", contents),
            ("/Resources", resources.inner),
        ])
        .inner
        .make_indirect();

    qpdf.add_page(&page, true).unwrap();

    let mut writer = qpdf.writer();
    writer
        .static_id(true)
        .force_pdf_version("1.7")
        .content_normalization(true)
        .preserve_unreferenced_objects(true)
        .object_stream_mode(ObjectStreamMode::Preserve)
        .linearize(true)
        .compress_streams(false)
        .stream_data_mode(StreamDataMode::Preserve);

    let mem = writer.write_to_memory().unwrap();
    writer.write("/tmp/test.pdf").unwrap();

    let mem_pdf = Qpdf::read_from_memory(&mem).unwrap();
    assert_eq!(mem_pdf.get_pdf_version(), "1.7");
    assert!(mem_pdf.is_linearized());
}

#[test]
fn test_qpdf_basic_objects() {
    let qpdf = Qpdf::empty();
    let obj = qpdf.new_bool(true);
    assert!(obj.is_bool() && obj.as_bool());
    assert_eq!(obj.to_string(), "true");

    let obj = qpdf.new_name("foo");
    assert!(obj.is_name() && obj.as_name() == "foo");
    assert_eq!(obj.to_string(), "foo");

    let obj = qpdf.new_integer(12_3456_7890);
    assert!(obj.is_scalar() && obj.as_i64() == 12_3456_7890);
    assert_eq!(obj.to_string(), "1234567890");

    let obj = qpdf.new_null();
    assert!(obj.is_null());
    assert_eq!(obj.to_string(), "null");

    let obj = qpdf.new_real(1.2345, 3);
    assert_eq!(obj.as_real(), "1.234");
    assert_eq!(obj.to_string(), "1.234");

    let obj = qpdf.new_uninitialized();
    assert!(!obj.is_initialized());

    let obj = qpdf.new_stream(&[]);
    assert!(obj.is_stream());
    assert_eq!(obj.to_string(), "3 0 R");

    let stream_dict = obj.get_stream_dictionary();
    stream_dict.set("/Type", &qpdf.new_name("/Stream"));

    let indirect = obj.make_indirect();
    assert!(indirect.is_indirect());
    assert_ne!(indirect.get_id(), obj.get_id());
    assert_eq!(indirect.get_generation(), obj.get_generation());
}

#[test]
fn test_qpdf_streams() {
    let qpdf = Qpdf::empty();

    let obj = qpdf.get_object_by_id(1234, 1);
    assert!(obj.is_none());

    let obj = qpdf.new_stream_with_dictionary([("/Type", qpdf.new_name("/Test"))], &[1, 2, 3, 4]);
    assert!(obj.is_stream());

    let by_id = qpdf
        .get_object_by_id(obj.get_id(), obj.get_generation())
        .unwrap();
    println!("{}", by_id.to_string());

    let data = by_id.get_stream_data(StreamDecodeLevel::None).unwrap();
    assert_eq!(data.as_ref(), &[1, 2, 3, 4]);

    let stream_dict = obj.get_stream_dictionary();
    assert_eq!(stream_dict.get("/Type").unwrap().as_name(), "/Test");

    let indirect = obj.make_indirect();
    assert!(indirect.is_indirect());
    assert_ne!(indirect.get_id(), 0);
    assert_eq!(indirect.get_generation(), 0);
}

#[test]
fn test_parse_object() {
    let text = "<< /Type /Page /Resources << /XObject null >> /MediaBox null /Contents null >>";
    let qpdf = Qpdf::empty();
    let obj = qpdf.parse_object(text).unwrap();
    assert!(obj.is_dictionary());
    println!("{}", obj.to_string());
    println!("version: {}", qpdf.get_pdf_version());
}

#[test]
fn test_error() {
    let qpdf = Qpdf::empty();
    assert!(qpdf.get_page(0).is_none());
    let result = qpdf.parse_object("<<--< /Type -- null >>");
    assert!(result.is_err());
    println!("{:?}", result);
}

#[test]
fn test_array() {
    let qpdf = Qpdf::empty();
    let mut arr = qpdf.new_array();
    arr.push(&qpdf.new_integer(1));
    arr.push(&qpdf.new_integer(2));
    arr.push(&qpdf.new_integer(3));
    assert_eq!(arr.inner().to_string(), "[ 1 2 3 ]");

    assert!(arr.get(10).is_none());

    assert_eq!(
        arr.iter().map(|v| v.as_i32()).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );

    arr.set(1, &qpdf.new_integer(5));
    assert_eq!(arr.inner().to_string(), "[ 1 5 3 ]");
}

#[test]
fn test_dictionary() {
    let qpdf = Qpdf::empty();
    let dict: QpdfDictionary = qpdf
        .parse_object("<< /Type /Page /Resources << /XObject null >> /MediaBox [1 2 3 4] /Contents (hello) >>")
        .unwrap()
        .into();

    let keys = dict.keys().into_iter().collect::<HashSet<_>>();
    assert_eq!(
        keys,
        ["/Type", "/Resources", "/MediaBox", "/Contents"]
            .into_iter()
            .map(|s| s.to_owned())
            .collect::<HashSet<_>>()
    );

    assert!(dict.get("/Type").unwrap().is_name());
    assert_eq!(dict.get("/Contents").unwrap().as_string(), "hello");

    let bval = qpdf.new_bool(true);
    dict.set("/MyKey", &bval);

    let setval = dict.get("/MyKey").unwrap();
    assert!(setval.as_bool());
    assert_ne!(bval, setval);

    dict.remove("/MyKey");
    assert!(dict.get("/MyKey").is_none());
}

#[test]
fn test_strings() {
    let qpdf = Qpdf::empty();
    let bin_str = qpdf.new_binary_string(&[1, 2, 3, 4]);
    assert_eq!(bin_str.to_string(), "<01020304>");

    let utf8_str = qpdf.new_utf8_string("привет");
    assert_eq!(utf8_str.to_string(), "<feff043f04400438043204350442>");

    let plain_str = qpdf.new_string("hello");
    assert_eq!(plain_str.to_string(), "(hello)");
    assert_eq!(plain_str.to_binary(), "<68656c6c6f>");
}

#[test]
fn test_pdf_ops() {
    let qpdf = load_pdf();
    println!("{:?}", qpdf.get_pdf_version());

    let trailer = qpdf.get_trailer().unwrap();
    println!("trailer: {}", trailer.inner.to_string());

    let root = qpdf.get_root().unwrap();
    println!("root: {}", root.inner.to_string());
    assert_eq!(root.get("/Type").unwrap().as_name(), "/Catalog");
    assert!(root.has("/Pages"));

    let pages = qpdf.get_pages().unwrap();
    assert_eq!(pages.len(), 2);

    for page in pages {
        let dict: QpdfDictionary = page.into();
        let keys = dict.keys();
        assert!(!keys.is_empty());
        println!("{:?}", keys);

        let data = dict.inner.get_page_content_data().unwrap();
        println!("{}", String::from_utf8_lossy(data.as_ref()));

        qpdf.add_page(&dict.inner.clone(), false).unwrap();
    }

    let buffer = qpdf.writer().write_to_memory().unwrap();
    let saved_pdf = Qpdf::read_from_memory(&buffer).unwrap();
    assert_eq!(saved_pdf.get_num_pages().unwrap(), 4);

    let pages = saved_pdf.get_pages().unwrap();
    for page in pages {
        saved_pdf.remove_page(&page).unwrap();
    }
    assert_eq!(saved_pdf.get_num_pages().unwrap(), 0);
}

#[test]
fn test_pdf_encrypted() {
    let qpdf = Qpdf::read("tests/data/encrypted.pdf");
    assert!(qpdf.is_err());
    println!("{:?}", qpdf);

    let qpdf = Qpdf::read_encrypted("tests/data/encrypted.pdf", "test");
    assert!(qpdf.is_ok());

    let data = std::fs::read("tests/data/encrypted.pdf").unwrap();
    let qpdf = Qpdf::read_from_memory_encrypted(&data, "test");
    assert!(qpdf.is_ok());
}
