/* Copyright (c) 2021 Jeremy Davis (jeremydavis519@gmail.com)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software
 * and associated documentation files (the "Software"), to deal in the Software without restriction,
 * including without limitation the rights to use, copy, modify, merge, publish, distribute,
 * sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or
 * substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
 * NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! This module defines the character encodings recognized by the HTML parser.

// Comments consisting entirely of quotations come directly from the HTML specification and serve as
// bookmarks in that searching for them word-for-word will lead to the relevant part of the spec.

/// Represents a possible character encoding that the parser might need to decode.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CharEncoding {
    /// UTF-8
    Utf8,
    /// UTF-16, big-endian
    Utf16Be,
    /// UTF-16, little-endian
    Utf16Le,
    /// IBM-866
    Ibm866,
    /// IBM-8859-2
    Iso8859_2,
    /// IBM-8859-3
    Iso8859_3,
    /// IBM-8859-4
    Iso8859_4,
    /// IBM-8859-5
    Iso8859_5,
    /// IBM-8859-6
    Iso8859_6,
    /// IBM-8859-7
    Iso8859_7,
    /// IBM-8859-8
    Iso8859_8,
    /// IBM-8859-8-I
    Iso8859_8I,
    /// IBM-8859-10
    Iso8859_10,
    /// IBM-8859-13
    Iso8859_13,
    /// IBM-8859-14
    Iso8859_14,
    /// IBM-8859-15
    Iso8859_15,
    /// IBM-8859-16
    Iso8859_16,
    /// KOI8-R
    Koi8R,
    /// KOI8-U
    Koi8U,
    /// macintosh
    Macintosh,
    /// Windows-874
    Windows874,
    /// Windows-1250
    Windows1250,
    /// Windows-1251
    Windows1251,
    /// Windows-1252
    Windows1252,
    /// Windows-1253
    Windows1253,
    /// Windows-1254
    Windows1254,
    /// Windows-1255
    Windows1255,
    /// Windows-1256
    Windows1256,
    /// Windows-1257
    Windows1257,
    /// Windows-1258
    Windows1258,
    /// x-mac-cyrillic
    XMacCyrillic,
    /// GBK
    Gbk,
    /// gb18030
    Gb18030,
    /// Big5
    Big5,
    /// EUC-JP
    EucJp,
    /// ISO-2022-JP
    Iso2022Jp,
    /// Shift_JIS
    ShiftJis,
    /// EUC-KR
    EucKr,
    /// replacement
    Replacement,
    /// x-user-defined
    XUserDefined
}

impl CharEncoding {
    // "Extracting a character encoding from a meta element"
    // This is used specifically for `<meta http-equiv="content-type" content="...">` tags.
    pub(crate) fn from_meta(content: &[u8]) -> Result<Self, ()> {
        let mut position = 0;
        loop {
            match content.windows(7).skip(position).position(|w| w.eq_ignore_ascii_case(b"charset")) {
                Some(p) => position += p + 7,
                None => return Err(())
            };

            match content.iter().skip(position).position(|&b| !b.is_ascii_whitespace()) {
                Some(p) => position += p,
                None => return Err(())
            };

            if content[position] != b'=' {
                // This is not the charset we're looking for.
                continue;
            }

            match content.iter().skip(position).position(|&b| !b.is_ascii_whitespace()) {
                Some(p) => position += p,
                None => return Err(())
            };

            match content[position] {
                quote @ b'"' | quote @ b'\'' => {
                    let end = content.iter().skip(position).position(|&b| b == quote)
                        .map(|len| position + len)
                        .unwrap_or(position); // Empty string if quote is unmatched
                    return CharEncoding::from_label(&content[position .. end]);
                },
                _ => {
                    let end = content.iter().skip(position).position(|&b| b.is_ascii_whitespace() || b == b';')
                        .map(|len| position + len)
                        .unwrap_or(content.len());
                    return CharEncoding::from_label(&content[position .. end]);
                }
            }
        }
    }

    // https://encoding.spec.whatwg.org/#concept-encoding-get
    pub(crate) fn from_label(label: &[u8]) -> Result<Self, ()> {
        // Trim the ASCII whitespace from the beginning and end of the label.
        let label_start = match label.iter().position(|&b| !b.is_ascii_whitespace()) {
            Some(p) => p,
            None => return Err(())
        };
        let label_end = match label.iter().rposition(|&b| !b.is_ascii_whitespace()) {
            Some(p) => p + 1,
            None => return Err(())
        };
        let label = &label[label_start .. label_end];

        macro_rules! match_ignore_ascii_case {
            (($var:expr) { $($($value:expr),+ => $encoding:expr;)* _ => $def_encoding:expr; }) => {
                $(
                    if $($var.eq_ignore_ascii_case($value) ||)+ false {
                        $encoding
                    } else
                )*
                {
                    $def_encoding
                }
            };
        }

        // Look up the appropriate encoding for this label.
        match_ignore_ascii_case! {
            (label) {
                b"unicode-1-1-utf-8",
                b"unicode11utf8",
                b"unicode20utf8",
                b"utf-8",
                b"utf8",
                b"x-unicode20utf8" => Ok(CharEncoding::Utf8);
                b"866",
                b"cp866",
                b"csibm866",
                b"ibm866" => Ok(CharEncoding::Ibm866);
                b"csisolatin2",
                b"iso-8859-2",
                b"iso-ir-101",
                b"iso8859-2",
                b"iso88592",
                b"iso_8859-2",
                b"iso_8859-2:1987",
                b"l2",
                b"latin2" => Ok(CharEncoding::Iso8859_2);
                b"csisolatin3",
                b"iso-8859-3",
                b"iso-ir-109",
                b"iso8859-3",
                b"iso88593",
                b"iso_8859-3",
                b"iso_8859-3:1988",
                b"l3",
                b"latin3" => Ok(CharEncoding::Iso8859_3);
                b"csisolatin4",
                b"iso-8859-4",
                b"iso-ir-110",
                b"iso8859-4",
                b"iso88594",
                b"iso_8859-4",
                b"iso_8859-4:1988",
                b"l4",
                b"latin4" => Ok(CharEncoding::Iso8859_4);
                b"csisolatincyrillic",
                b"cyrillic",
                b"iso-8859-5",
                b"iso-ir-144",
                b"iso8859-5",
                b"iso88595",
                b"iso_8859-5",
                b"iso_8859-5:1988" => Ok(CharEncoding::Iso8859_5);
                b"arabic",
                b"asmo-708",
                b"csiso88596e",
                b"csiso88596i",
                b"csisolatinarabic",
                b"ecma-114",
                b"iso-8859-6",
                b"iso-8859-6-e",
                b"iso-8859-6-i",
                b"iso-ir-127",
                b"iso8859-6",
                b"iso88596",
                b"iso_8859-6",
                b"iso_8859-6:1987" => Ok(CharEncoding::Iso8859_6);
                b"csisolatingreek",
                b"ecma-118",
                b"elot_928",
                b"greek",
                b"greek8",
                b"iso-8859-7",
                b"iso-ir-126",
                b"iso8859-7",
                b"iso88597",
                b"iso_8859-7",
                b"iso_8859-7:1987",
                b"sun_eu_greek" => Ok(CharEncoding::Iso8859_7);
                b"csiso88598e",
                b"csisolatinhebrew",
                b"hebrew",
                b"iso-8859-8",
                b"iso-8859-8-e",
                b"iso-ir-138",
                b"iso8859-8",
                b"iso88598",
                b"iso_8859-8",
                b"iso_8859-8:1988",
                b"visual" => Ok(CharEncoding::Iso8859_8);
                b"csiso88598i",
                b"iso-8859-8-i",
                b"logical" => Ok(CharEncoding::Iso8859_8I);
                b"csisolatin6",
                b"iso-8859-10",
                b"iso-ir-157",
                b"iso8859-10",
                b"iso885910",
                b"l6",
                b"latin6" => Ok(CharEncoding::Iso8859_10);
                b"iso-8859-13",
                b"iso8859-13",
                b"iso885913" => Ok(CharEncoding::Iso8859_13);
                b"iso-8859-14",
                b"iso8859-14",
                b"iso885914" => Ok(CharEncoding::Iso8859_14);
                b"csisolatin9",
                b"iso-8859-15",
                b"iso8859-15",
                b"iso885915",
                b"iso_8859-15",
                b"l9" => Ok(CharEncoding::Iso8859_15);
                b"iso_8859-16" => Ok(CharEncoding::Iso8859_16);
                b"cskoi8r",
                b"koi",
                b"koi8",
                b"koi8-r",
                b"koi8_r" => Ok(CharEncoding::Koi8R);
                b"koi8-ru",
                b"koi8-u" => Ok(CharEncoding::Koi8U);
                b"csmacintosh",
                b"mac",
                b"macintosh",
                b"x-mac-roman" => Ok(CharEncoding::Macintosh);
                b"dos-874",
                b"iso-8859-11",
                b"iso8859-11",
                b"iso885911",
                b"tis-620",
                b"windows-874" => Ok(CharEncoding::Windows874);
                b"cp1250",
                b"windows-1250",
                b"x-cp1250" => Ok(CharEncoding::Windows1250);
                b"cp1251",
                b"windows-1251",
                b"x-cp1251" => Ok(CharEncoding::Windows1251);
                b"ansi_x3.4-1968",
                b"ascii",
                b"cp1252",
                b"cp819",
                b"csisolatin1",
                b"ibm819",
                b"iso-8859-1",
                b"iso-ir-100",
                b"iso8859-1",
                b"iso88591",
                b"iso_8859-1",
                b"iso_8859-1:1987",
                b"l1",
                b"latin1",
                b"us-ascii",
                b"windows-1252",
                b"x-cp1252" => Ok(CharEncoding::Windows1252);
                b"cp1253",
                b"windows-1253",
                b"x-cp1253" => Ok(CharEncoding::Windows1253);
                b"cp1254",
                b"csisolatin5",
                b"iso-8859-9",
                b"iso-ir-148",
                b"iso8859-9",
                b"iso88599",
                b"iso_8859-9",
                b"iso_8859-9:1989",
                b"l5",
                b"latin5",
                b"windows-1254",
                b"x-cp1254" => Ok(CharEncoding::Windows1254);
                b"cp1255",
                b"windows-1255",
                b"x-cp1255" => Ok(CharEncoding::Windows1255);
                b"cp1256",
                b"windows-1256",
                b"x-cp1256" => Ok(CharEncoding::Windows1256);
                b"cp1257",
                b"windows-1257",
                b"x-cp1257" => Ok(CharEncoding::Windows1257);
                b"cp1258",
                b"windows-1258",
                b"x-cp1258" => Ok(CharEncoding::Windows1258);
                b"x-mac-cyrillic",
                b"x-mac-ukrainian" => Ok(CharEncoding::XMacCyrillic);
                b"chinese",
                b"csgb2312",
                b"csiso58gb231280",
                b"gb2312",
                b"gb_2312",
                b"gb_2312-80",
                b"gbk",
                b"iso-ir-58",
                b"x-gbk" => Ok(CharEncoding::Gbk);
                b"gb18030" => Ok(CharEncoding::Gb18030);
                b"big5",
                b"big5-hkscs",
                b"cn-big5",
                b"csbig5",
                b"x-x-big5" => Ok(CharEncoding::Big5);
                b"cseucpkdfmtjapanese",
                b"euc-jp",
                b"x-euc-jp" => Ok(CharEncoding::EucJp);
                b"csiso2022jp",
                b"iso-2022-jp" => Ok(CharEncoding::Iso2022Jp);
                b"csshiftjis",
                b"ms932",
                b"ms_kanji",
                b"shift-jis",
                b"shift_jis",
                b"sjis",
                b"windows-31j",
                b"x-sjis" => Ok(CharEncoding::ShiftJis);
                b"cseuckr",
                b"csksc56011987",
                b"euc-kr",
                b"iso-ir-149",
                b"korean",
                b"ks_c_5601-1987",
                b"ks_c_5601-1989",
                b"ksc5601",
                b"ksc_5601",
                b"windows-949" => Ok(CharEncoding::EucKr);
                b"csiso2022kr",
                b"hz-gb-2312",
                b"iso-2022-cn",
                b"iso-2022-cn-ext",
                b"iso-2022-kr",
                b"replacement" => Ok(CharEncoding::Replacement);
                b"unicodefffe",
                b"utf-16be" => Ok(CharEncoding::Utf16Be);
                b"csunicode",
                b"iso-10646-ucs-2",
                b"ucs-2",
                b"unicode",
                b"unicodefeff",
                b"utf-16",
                b"utf-16le" => Ok(CharEncoding::Utf16Le);
                b"x-user-defined" => Ok(CharEncoding::XUserDefined);
                _ => Err(()); // Unrecognized encoding label
            }
        }
    }
}

// Represents the parser's level of confidence that it has the right character encoding. This is
// always paired with a [`CharEncoding`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum CharEncodingConfidence {
    /// The data stream doesn't have a definite encoding, but it looks like it might be the given
    /// encoding.
    Tentative,
    /// The data stream has a definite encoding. There is no chance that we're wrong about it.
    Certain,
    /// It doesn't matter what the character encoding is. This confidence level is used internally
    /// when Unicode code points are fed to the parser rather than bytes.
    Irrelevant
}

// Represents a character encoding that may or may not exist and which we may or may not have
// already tried to find.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum MaybeCharEncoding {
    Some(CharEncoding),
    None,
    Tbd
}