use unic_langid_impl::LanguageIdentifier;
use unic_locale_impl::parser::parse_locale;
use unic_locale_impl::{CharacterDirection, ExtensionsMap, Locale};

fn assert_locale_extensions(loc: &Locale, extensions: &ExtensionsMap) {
    assert_eq!(&loc.extensions, extensions);
}

fn assert_parsed_locale_identifier(input: &str, extensions: &ExtensionsMap) {
    let loc = parse_locale(input).unwrap();
    assert_locale_extensions(&loc, extensions);
}

#[test]
fn test_basic() {
    let loc: Locale = "en-US".parse().unwrap();
    let loc2 = Locale {
        id: LanguageIdentifier::from_parts(
            "en".parse().unwrap(),
            None,
            Some("US".parse().unwrap()),
            &[],
        ),
        extensions: ExtensionsMap::default(),
    };
    assert_eq!(loc, loc2);
}

#[test]
fn test_from_parts() {
    let extensions = ExtensionsMap::default();
    let loc = Locale::from_parts("en".parse().unwrap(), None, None, &[], Some(extensions));
    let loc2 = Locale {
        id: LanguageIdentifier::from_parts("en".parse().unwrap(), None, None, &[]),
        extensions: ExtensionsMap::default(),
    };
    assert_eq!(loc, loc2);
}

#[test]
fn test_locale_identifier() {
    let mut extensions = ExtensionsMap::default();
    extensions.unicode.set_keyword("hc", &["h12"]).unwrap();
    assert_parsed_locale_identifier("pl-u-hc-h12", &extensions);

    extensions.unicode.set_attribute("foo").unwrap();
    assert_parsed_locale_identifier("pl-u-foo-hc-h12", &extensions);

    let val = extensions
        .unicode
        .keyword("hc")
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(val, &["h12"]);

    let val = extensions
        .unicode
        .keyword("aa")
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(val.is_empty(), true);

    let val = extensions.unicode.remove_keyword("hc").unwrap();
    assert_eq!(val, true);
    assert_parsed_locale_identifier("pl-u-foo", &extensions);

    let val = extensions.unicode.has_attribute("foo").unwrap();
    assert_eq!(val, true);

    let val = extensions.unicode.has_attribute("aaa").unwrap();
    assert_eq!(val, false);

    let val = extensions.unicode.remove_attribute("foo").unwrap();
    assert_eq!(val, true);
    assert_parsed_locale_identifier("pl", &extensions);

    extensions.transform.set_tfield("m0", &["foo"]).unwrap();
    assert_parsed_locale_identifier("pl-t-m0-foo", &extensions);

    let val = extensions
        .transform
        .tfield("m0")
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(val, &["foo"]);

    let val = extensions
        .transform
        .tfield("x0")
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(val.is_empty(), true);

    let val = extensions.transform.remove_tfield("m0").unwrap();
    assert_eq!(val, true);
    assert_parsed_locale_identifier("pl", &extensions);

    let mut extensions = ExtensionsMap::default();
    extensions.private.add_tag("testing").unwrap();
    assert_parsed_locale_identifier("und-x-testing", &extensions);
}

#[test]
fn test_serialize_locale() {
    let loc: Locale = "en-u-hc-h12".parse().unwrap();
    assert_eq!(&loc.to_string(), "en-u-hc-h12");
}

#[test]
fn test_from_langid() {
    let langid: LanguageIdentifier = "en-US".parse().unwrap();
    let loc = Locale::from(langid);
    assert_eq!(&loc.to_string(), "en-US");
}

#[test]
fn test_to_langid() {
    let loc: Locale = "en-US-u-hc-h12".parse().unwrap();
    let langid: LanguageIdentifier = loc.into();
    assert_eq!(langid.to_string(), "en-US");
}

// #[test]
// fn test_from_parts_unchecked() {
//     let loc: Locale = "en-US".parse().unwrap();
//     let (lang, script, region, variants, extensions) = loc.into_parts();
//     let loc = Locale::from_raw_parts_unchecked(
//         lang.map(|l| unsafe { TinyStr8::new_unchecked(l) }),
//         script.map(|s| unsafe { TinyStr4::new_unchecked(s) }),
//         region.map(|r| unsafe { TinyStr4::new_unchecked(r) }),
//         variants.map(|v| {
//             v.into_iter()
//                 .map(|v| unsafe { TinyStr8::new_unchecked(*v) })
//                 .collect()
//         }),
//         extensions.parse().unwrap(),
//     );
//     assert_eq!(&loc.to_string(), "en-US");
// }

#[test]
fn test_matches() {
    let loc_en: Locale = "en-u-hc-h12".parse().unwrap();
    let loc_en_us: Locale = "en-US".parse().unwrap();
    let loc_en_us2: Locale = "en-US-u-hc-h24".parse().unwrap();
    let loc_pl: Locale = "pl".parse().unwrap();
    assert_eq!(loc_en.matches(&loc_en_us, false, false), false);
    assert_eq!(loc_en_us.matches(&loc_en_us2, false, false), true);
    assert_eq!(loc_en.matches(&loc_pl, false, false), false);
    assert_eq!(loc_en.matches(&loc_en_us, true, false), true);

    let langid_en: LanguageIdentifier = "en-US".parse().unwrap();
    assert_eq!(langid_en.matches(&loc_en_us, true, true), true);
    assert_eq!(
        loc_en_us.matches(&Locale::from(langid_en), true, true),
        true
    );
}

#[test]
fn test_set_fields() {
    let mut loc = Locale::default();
    assert_eq!(&loc.to_string(), "und");

    loc.id.language = "pl".parse().expect("Setting language failed");
    assert_eq!(&loc.to_string(), "pl");

    loc.id.language = "de".parse().expect("Setting language failed");
    assert_eq!(&loc.to_string(), "de");
    loc.id.region = Some("AT".parse().expect("Setting region failed"));
    assert_eq!(&loc.to_string(), "de-AT");
    loc.id.script = Some("Latn".parse().expect("Setting script failed"));
    assert_eq!(&loc.to_string(), "de-Latn-AT");
    loc.id
        .set_variants(&["macos".parse().expect("Setting variants failed")]);
    assert_eq!(&loc.to_string(), "de-Latn-AT-macos");

    loc.id.language.clear();
    assert_eq!(&loc.to_string(), "und-Latn-AT-macos");
    loc.id.region = None;
    assert_eq!(&loc.to_string(), "und-Latn-macos");
    loc.id.script = None;
    assert_eq!(&loc.to_string(), "und-macos");
    loc.id.clear_variants();
    assert_eq!(&loc.to_string(), "und");
}

#[cfg(feature = "likelysubtags")]
#[test]
fn test_likelysubtags() {
    let mut loc_en: Locale = "en-u-hc-h12".parse().unwrap();
    assert_eq!(loc_en.id.maximize(), true);
    assert_eq!(loc_en.to_string(), "en-Latn-US-u-hc-h12");

    let mut loc_sr: Locale = "sr-Cyrl-u-hc-h12".parse().unwrap();
    assert_eq!(loc_sr.id.maximize(), true);
    assert_eq!(loc_sr.to_string(), "sr-Cyrl-RS-u-hc-h12");

    let mut loc_zh_hans: Locale = "zh-Hans-u-hc-h12".parse().unwrap();
    assert_eq!(loc_zh_hans.id.minimize(), true);
    assert_eq!(loc_zh_hans.to_string(), "zh-u-hc-h12");

    let mut loc_zh_hant: Locale = "zh-Hant-u-hc-h12".parse().unwrap();
    assert_eq!(loc_zh_hant.id.minimize(), true);
    assert_eq!(loc_zh_hant.to_string(), "zh-TW-u-hc-h12");
}

#[test]
fn test_character_direction() {
    let loc_en: Locale = "en-u-hc-h12".parse().unwrap();
    assert_eq!(loc_en.id.character_direction(), CharacterDirection::LTR);

    let loc_ar: Locale = "ar-AF-u-hc-h12".parse().unwrap();
    assert_eq!(loc_ar.id.character_direction(), CharacterDirection::RTL);

    let loc_mn: Locale = "mn-Mong".parse().unwrap();
    assert_eq!(loc_mn.id.character_direction(), CharacterDirection::TTB);
}

#[test]
fn test_unicode_attributes_ordering() {
    let mut loc: Locale = "en-u-foo-bar".parse().unwrap();
    assert_eq!(&loc.to_string(), "en-u-bar-foo");

    loc.extensions
        .unicode
        .set_attribute("baz")
        .expect("Can't set attribute");
    assert_eq!(&loc.to_string(), "en-u-bar-baz-foo");
}

#[test]
fn test_other_extensions() {
    let inputs = [
        ("en-US", "en-US"),
        ("en-a-aaa", "en-a-aaa"),
        ("en-US-b-foo", "en-US-b-foo"),
        ("en-0-001", "en-0-001"),
        (
            "en-US-b-foo-a-bar-u-ca-buddhist",
            "en-US-a-bar-b-foo-u-ca-buddhist",
        ),
        ("und-a-xyz-x-test", "und-a-xyz-x-test"),
        ("en-v-foo-w-bar", "en-v-foo-w-bar"),
        ("en-a-warbl-babble", "en-a-warbl-babble"),
        ("en-a-foo-b-bar-a-baz", "en-a-foo-baz-b-bar"),
    ];

    for (input, expected) in &inputs {
        let loc: Locale = input.parse().expect("Parsing failed");
        assert_eq!(&loc.to_string(), expected);
    }
}

#[test]
fn test_invalid_extensions_no_panic() {
    let repro_inputs = [
        ("en-US", Some("en-US")),
        ("en-a", Some("en")),
        ("en-a-aaa", Some("en-a-aaa")),
        ("en-0", Some("en")),
        ("und-b-x", Some("und")),
        ("en-@", None),
        ("en-US-b-foo", Some("en-US-b-foo")),
        ("en-US-invalidextension", None),
        ("en-@-foo", None),
    ];

    for (input, expected) in &repro_inputs {
        let result = input.parse::<Locale>();
        match expected {
            Some(expected_str) => {
                let loc = result.expect("Should parse without panic");
                assert_eq!(&loc.to_string(), expected_str);
            }
            None => {
                assert!(result.is_err(), "Expected parse error for {}", input);
            }
        }
    }
}

#[test]
fn test_other_extensions_defensive_display() {
    let mut loc: Locale = "en-US-u-ca-buddhist-t-en-h0-hybrid"
        .parse()
        .expect("Should parse");

    // Programmatically insert into public `other` map with various singletons including uppercase and reserved letters
    let tag_v: tinystr::TinyStr8 = "valv".parse().unwrap();
    let tag_x: tinystr::TinyStr8 = "valx".parse().unwrap();
    let tag_a: tinystr::TinyStr8 = "vala".parse().unwrap();
    let tag_t: tinystr::TinyStr8 = "valt".parse().unwrap();

    loc.extensions.other.insert('V', vec![tag_v]); // Uppercase V > u, should format as lowercase after -u-
    loc.extensions.other.insert('X', vec![tag_x]); // Uppercase X, private use, should format at the very end after standard -x-
    loc.extensions.other.insert('A', vec![tag_a]); // Uppercase A < t, should format before -t-
    loc.extensions.other.insert('t', vec![tag_t]); // Lowercase t in other map, should format defensively alongside transform

    assert_eq!(
        &loc.to_string(),
        "en-US-a-vala-t-en-h0-hybrid-t-valt-u-ca-buddhist-v-valv-x-valx"
    );
}

#[test]
fn test_transform_to_unicode_sequencing() {
    let inputs = [
        // BCP 47 canonical order: -t- before -u-, which previously errored when -t- ended with a tvalue
        (
            "en-US-t-h0-hybrid-u-ca-buddhist",
            "en-US-t-h0-hybrid-u-ca-buddhist",
        ),
        ("pl-t-en-m0-val-u-hc-h12", "pl-t-en-m0-val-u-hc-h12"),
        (
            "en-US-t-h0-hybrid-a-foo-u-ca-buddhist-x-bar",
            "en-US-a-foo-t-h0-hybrid-u-ca-buddhist-x-bar",
        ),
    ];

    for (input, expected) in &inputs {
        let loc: Locale = input.parse().expect("Parsing failed");
        assert_eq!(&loc.to_string(), *expected);
    }
}

#[test]
fn test_other_extensions_defensive_sorting() {
    let mut loc: Locale = "en-US".parse().expect("Should parse");

    // Test case-insensitive sorting in BTreeMap: in raw ASCII, 'B' (66) comes before 'a' (97).
    // Without sorting by lowercase in Display, this would emit `-b` before `-a`, violating BCP 47.
    let tag_a: tinystr::TinyStr8 = "vala".parse().unwrap();
    let tag_b: tinystr::TinyStr8 = "valb".parse().unwrap();
    let tag_0: tinystr::TinyStr8 = "val0".parse().unwrap();

    loc.extensions.other.insert('B', vec![tag_b]); // Uppercase 'B' (ASCII 66)
    loc.extensions.other.insert('a', vec![tag_a]); // Lowercase 'a' (ASCII 97)
    loc.extensions.other.insert('0', vec![tag_0]); // Digit '0' (ASCII 48)

    assert_eq!(&loc.to_string(), "en-US-0-val0-a-vala-b-valb");
}
