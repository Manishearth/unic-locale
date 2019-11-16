use crate::CharacterDirection;
use crate::LanguageIdentifier;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tinystr::{TinyStr4, TinyStr8};

// Layout

pub fn get_layout_entry(path: PathBuf) -> Option<(String, String, CharacterDirection)> {
    let contents = fs::read_to_string(path).expect("Something went wrong reading the file");
    let v: Value = serde_json::from_str(&contents).unwrap();

    let langid_key = v["main"].as_object().unwrap().keys().nth(0).unwrap();

    if langid_key == "root" {
        return None;
    }

    let character_order = match v["main"][langid_key]["layout"]["orientation"]["characterOrder"]
        .as_str()
        .unwrap()
    {
        "right-to-left" => CharacterDirection::RTL,
        "left-to-right" => CharacterDirection::LTR,
        _ => unimplemented!("Encountered unknown directionality!"),
    };

    let version = v["main"][langid_key]["identity"]["version"]["_cldrVersion"]
        .as_str()
        .unwrap()
        .to_string();
    Some((langid_key.to_string(), version, character_order))
}

fn get_langid_to_direction_map(
    path: PathBuf,
) -> HashMap<LanguageIdentifier, (String, CharacterDirection)> {
    let mut result = HashMap::new();
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let mut path = entry.path();
        path.push("layout.json");

        if let Some((langid_key, version, character_order)) = get_layout_entry(path) {
            let langid: LanguageIdentifier = langid_key.parse().unwrap();
            result.insert(langid, (version, character_order));
        }
    }
    result
}

fn check_all_variants_rtl(
    map: &HashMap<LanguageIdentifier, (String, CharacterDirection)>,
    lang: &str,
) -> bool {
    for (langid, (_, dir)) in map.iter() {
        if langid.get_language() == lang && dir != &CharacterDirection::RTL {
            return false;
        }
    }
    true
}

pub fn generate_layout(path: &str) -> Result<(String, String), std::fmt::Error> {
    let path = Path::new(path).join("main");
    let map = get_langid_to_direction_map(path);

    let mut langs = vec![];

    let mut version = None;

    for (langid, (ver, dir)) in map.iter() {
        if let Some(ref version) = version {
            if version != ver {
                panic!("All CLDR data must use the same version!");
            }
        } else {
            version = Some(ver.clone());
        }

        if dir == &CharacterDirection::LTR {
            continue;
        }

        let lang = langid.get_language().to_string();

        assert!(
            check_all_variants_rtl(&map, &lang),
            "We didn't expect a language with two directionalities!"
        );
        if !langs.contains(&lang) {
            langs.push(lang.to_string());
        }
    }

    let mut u64_list: Vec<u64> = langs
        .iter()
        .map(|s| TinyStr8::from_str(s).unwrap().into())
        .collect();

    u64_list.sort();

    let list: Vec<String> = u64_list.iter().map(|s| s.to_string()).collect();

    let mut result = String::new();

    writeln!(
        result,
        "pub const CHARACTER_DIRECTION_RTL: [u64; {}] = [{}];",
        list.len(),
        list.join(", ")
    )?;
    writeln!(result, "pub fn is_rtl(subtag: u64) -> bool {{")?;
    writeln!(
        result,
        "    CHARACTER_DIRECTION_RTL.binary_search(&subtag).is_ok()"
    )?;
    writeln!(result, "}}")?;

    let version = version.expect("CLDR Version should be specified.");
    Ok((version, result))
}

// Likely Subtags

type LangIdSubTags = (Option<u64>, Option<u32>, Option<u32>);

fn serialize_val(input: LangIdSubTags) -> String {
    format!(
        "({}, {}, {})",
        serialize_lang_option(input.0),
        serialize_script_option(input.1),
        serialize_region_option(input.2)
    )
}

fn serialize_lang_option(l: Option<u64>) -> String {
    if let Some(l) = l {
        format!("Some({})", l)
    } else {
        String::from("None")
    }
}

fn serialize_script_option(r: Option<u32>) -> String {
    if let Some(r) = r {
        format!("Some({})", r)
    } else {
        String::from("None")
    }
}

fn serialize_region_option(r: Option<u32>) -> String {
    if let Some(r) = r {
        format!("Some({})", r)
    } else {
        String::from("None")
    }
}

mod ast {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Resource<'s> {
        #[serde(borrow)]
        pub supplemental: Supplemental<'s>,
    }

    #[derive(Deserialize)]
    pub struct Supplemental<'s> {
        #[serde(borrow)]
        pub version: Version<'s>,

        #[serde(rename = "likelySubtags")]
        #[serde(borrow)]
        #[serde(with = "tuple_vec_map")]
        pub likely_subtags: Vec<(&'s [u8], &'s [u8])>,
    }

    #[derive(Deserialize)]
    pub struct Version<'s> {
        #[serde(rename = "_cldrVersion")]
        #[serde(borrow)]
        pub cldr_version: &'s str,
    }
}

pub fn get_likely_subtags_data(
    path: &str,
) -> (
    String,
    Vec<(u64, LangIdSubTags)>,
    Vec<(u64, u32, LangIdSubTags)>,
    Vec<(u64, u32, LangIdSubTags)>,
    Vec<(u32, u32, LangIdSubTags)>,
    Vec<(u32, LangIdSubTags)>,
    Vec<(u32, LangIdSubTags)>,
) {
    let path = Path::new(path)
        .join("supplemental")
        .join("likelySubtags.json");
    let contents = fs::read_to_string(path).expect("Something went wrong reading the file");
    let v: ast::Resource = serde_json::from_str(&contents).unwrap();
    let values = v.supplemental.likely_subtags;

    let mut lang_only: Vec<(u64, LangIdSubTags)> = vec![];
    let mut lang_region: Vec<(u64, u32, LangIdSubTags)> = vec![];
    let mut lang_script: Vec<(u64, u32, LangIdSubTags)> = vec![];
    let mut script_region: Vec<(u32, u32, LangIdSubTags)> = vec![];
    let mut region_only: Vec<(u32, LangIdSubTags)> = vec![];
    let mut script_only: Vec<(u32, LangIdSubTags)> = vec![];

    for (k, v) in values {
        let key_langid = LanguageIdentifier::from_bytes(k).expect("Failed to parse a key.");
        let mut value_langid = LanguageIdentifier::from_bytes(v).expect("Failed to parse a value.");
        if let Some("ZZ") = value_langid.get_region() {
            value_langid.clear_region();
        }
        let (val_lang, val_script, val_region, _) = value_langid.into_raw_parts();

        let (key_lang, key_script, key_region, _) = key_langid.into_raw_parts();

        match (key_lang, key_script, key_region) {
            (Some(l), None, None) => lang_only.push((l, (val_lang, val_script, val_region))),
            (Some(l), None, Some(r)) => {
                lang_region.push((l, r, (val_lang, val_script, val_region)))
            }
            (Some(l), Some(s), None) => {
                lang_script.push((l, s, (val_lang, val_script, val_region)))
            }
            (None, Some(s), Some(r)) => {
                script_region.push((s, r, (val_lang, val_script, val_region)))
            }
            (None, Some(s), None) => script_only.push((s, (val_lang, val_script, val_region))),
            (None, None, Some(r)) => region_only.push((r, (val_lang, val_script, val_region))),
            (None, None, None) => {
                // XXX: We want to handle "und"!
            }
            _ => {
                panic!("Unknown scenario: {:#?}", std::str::from_utf8(k));
            }
        }
    }

    lang_only.sort_by_key(|a| a.0);
    lang_region.sort_by_key(|a| (a.0, a.1));
    lang_script.sort_by_key(|a| (a.0, a.1));
    script_region.sort_by_key(|a| (a.0, a.1));
    script_only.sort_by_key(|a| a.0);
    region_only.sort_by_key(|a| a.0);

    let version = v.supplemental.version.cldr_version.to_string();

    (
        version,
        lang_only,
        lang_region,
        lang_script,
        script_region,
        region_only,
        script_only,
    )
}

pub fn generate_likely_subtags(path: &str) -> Result<(String, String), std::fmt::Error> {
    let (version, lang_only, lang_region, lang_script, script_region, region_only, script_only) =
        get_likely_subtags_data(path);

    let mut result = String::new();

    writeln!(result, "#![allow(clippy::type_complexity)]")?;
    writeln!(result, "#![allow(clippy::unreadable_literal)]\n")?;

    writeln!(
        result,
        "pub const LANG_ONLY: &[(u64, (Option<u64>, Option<u32>, Option<u32>)); {}] = &[",
        lang_only.len()
    )?;
    for (key_lang, val) in lang_only {
        writeln!(result, "    ({}, {}),", key_lang, serialize_val(val),)?;
    }
    writeln!(result, "];")?;

    writeln!(
        result,
        "pub const LANG_REGION: [(u64, u32, (Option<u64>, Option<u32>, Option<u32>)); {}] = [",
        lang_region.len()
    )?;
    for (key_lang, key_region, val) in lang_region {
        writeln!(
            result,
            "    ({}, {}, {}),",
            key_lang,
            key_region,
            serialize_val(val),
        )?;
    }
    writeln!(result, "];")?;
    writeln!(
        result,
        "pub const LANG_SCRIPT: [(u64, u32, (Option<u64>, Option<u32>, Option<u32>)); {}] = [",
        lang_script.len()
    )?;
    for (key_lang, key_script, val) in lang_script {
        writeln!(
            result,
            "    ({}, {}, {}),",
            key_lang,
            key_script,
            serialize_val(val),
        )?;
    }
    writeln!(result, "];")?;
    writeln!(
        result,
        "pub const SCRIPT_REGION: [(u32, u32, (Option<u64>, Option<u32>, Option<u32>)); {}] = [",
        script_region.len()
    )?;
    for (key_script, key_region, val) in script_region {
        writeln!(
            result,
            "    ({}, {}, {}),",
            key_script,
            key_region,
            serialize_val(val),
        )?;
    }
    writeln!(result, "];")?;
    writeln!(
        result,
        "pub const SCRIPT_ONLY: [(u32, (Option<u64>, Option<u32>, Option<u32>)); {}] = [",
        script_only.len()
    )?;
    for (key_script, val) in script_only {
        writeln!(result, "    ({}, {}),", key_script, serialize_val(val),)?;
    }
    writeln!(result, "];")?;
    writeln!(
        result,
        "pub const REGION_ONLY: [(u32, (Option<u64>, Option<u32>, Option<u32>)); {}] = [",
        region_only.len()
    )?;
    for (key_region, val) in region_only {
        writeln!(result, "    ({}, {}),", key_region, serialize_val(val),)?;
    }
    writeln!(result, "];")?;
    Ok((version, result))
}
