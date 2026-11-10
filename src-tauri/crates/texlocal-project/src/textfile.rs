//! Leitura e escrita de arquivos de texto com detecção de codificação e fim de linha (TL-301, ADR 26).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Eol {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Encoding {
    #[serde(rename = "utf8")]
    Utf8,
    /// ISO-8859-1. Também é o fallback quando os bytes não são UTF-8 válido.
    #[serde(rename = "latin1")]
    Latin1,
    /// Windows-1252 (o "ANSI" do Windows no Ocidente).
    #[serde(rename = "windows1252")]
    Windows1252,
    /// ISO-8859-15 (Latin-9: como o Latin-1, com € e outras letras).
    #[serde(rename = "iso885915")]
    Iso885915,
}

const CP1252_HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

/// Caractere de um byte numa codificação de 8 bits.
fn byte_to_char(encoding: Encoding, b: u8) -> char {
    let code = match (encoding, b) {
        (Encoding::Windows1252, 0x80..=0x9F) => u32::from(CP1252_HIGH[usize::from(b - 0x80)]),
        (Encoding::Iso885915, 0xA4) => 0x20AC,
        (Encoding::Iso885915, 0xA6) => 0x0160,
        (Encoding::Iso885915, 0xA8) => 0x0161,
        (Encoding::Iso885915, 0xB4) => 0x017D,
        (Encoding::Iso885915, 0xB8) => 0x017E,
        (Encoding::Iso885915, 0xBC) => 0x0152,
        (Encoding::Iso885915, 0xBD) => 0x0153,
        (Encoding::Iso885915, 0xBE) => 0x0178,
        _ => u32::from(b),
    };
    char::from_u32(code).unwrap_or('\u{FFFD}')
}

fn char_to_byte(encoding: Encoding, c: char) -> Option<u8> {
    if c.is_ascii() {
        return u8::try_from(u32::from(c)).ok();
    }
    (0x80..=0xFFu8).find(|&b| byte_to_char(encoding, b) == c)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decoded {
    /// Texto com fins de linha normalizados para LF.
    pub text: String,
    pub eol: Eol,
    pub encoding: Encoding,
    pub bom: bool,
}

/// Decodifica bytes. O fim de linha dominante é preservado em `eol`; o texto sai em LF.
pub fn decode(bytes: &[u8]) -> Decoded {
    decode_as(bytes, None)
}

/// Como [`decode`], mas com a codificação escolhida pelo usuário (`None` = detectar).
pub fn decode_as(bytes: &[u8], forced: Option<Encoding>) -> Decoded {
    let (body, bom) = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => (rest, true),
        None => (bytes, false),
    };
    let (raw, encoding) = match forced {
        Some(Encoding::Utf8) => (String::from_utf8_lossy(body).into_owned(), Encoding::Utf8),
        Some(enc) => (body.iter().map(|&b| byte_to_char(enc, b)).collect(), enc),
        None => match std::str::from_utf8(body) {
            Ok(s) => (s.to_owned(), Encoding::Utf8),
            Err(_) => (body.iter().map(|&b| b as char).collect(), Encoding::Latin1),
        },
    };
    let crlf = raw.matches("\r\n").count();
    let lf = raw.matches('\n').count() - crlf;
    // Arquivo novo/vazio ou sem quebras: LF (ADR 26).
    let eol = if crlf > lf { Eol::Crlf } else { Eol::Lf };
    let text = if crlf > 0 {
        raw.replace("\r\n", "\n")
    } else {
        raw
    };
    Decoded {
        text,
        eol,
        encoding,
        bom,
    }
}

/// Codifica de volta. Falha com o primeiro caractere não representável na codificação.
pub fn encode(text: &str, eol: Eol, encoding: Encoding, bom: bool) -> Result<Vec<u8>, char> {
    let body = match eol {
        Eol::Lf => text.to_owned(),
        Eol::Crlf => text.replace('\n', "\r\n"),
    };
    let mut out = Vec::with_capacity(body.len() + 3);
    match encoding {
        Encoding::Utf8 => {
            if bom {
                out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
            }
            out.extend_from_slice(body.as_bytes());
        }
        single_byte => {
            for c in body.chars() {
                out.push(char_to_byte(single_byte, c).ok_or(c)?);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_crlf_e_normaliza() {
        let d = decode(b"a\r\nb\r\nc");
        assert_eq!(d.eol, Eol::Crlf);
        assert_eq!(d.text, "a\nb\nc");
        assert_eq!(
            encode(&d.text, d.eol, d.encoding, d.bom).unwrap(),
            b"a\r\nb\r\nc"
        );
    }

    #[test]
    fn vazio_e_lf_usam_lf() {
        assert_eq!(decode(b"").eol, Eol::Lf);
        assert_eq!(decode(b"a\nb\n").eol, Eol::Lf);
    }

    #[test]
    fn misto_usa_o_dominante() {
        assert_eq!(decode(b"a\r\nb\nc\nd\n").eol, Eol::Lf);
        assert_eq!(decode(b"a\r\nb\r\nc\n").eol, Eol::Crlf);
    }

    #[test]
    fn latin1_e_detectado_e_preservado() {
        let bytes = b"caf\xe9\n";
        let d = decode(bytes);
        assert_eq!(d.encoding, Encoding::Latin1);
        assert_eq!(d.text, "café\n");
        assert_eq!(encode(&d.text, d.eol, d.encoding, d.bom).unwrap(), bytes);
    }

    #[test]
    fn latin1_rejeita_caractere_fora_da_faixa() {
        assert_eq!(encode("日", Eol::Lf, Encoding::Latin1, false), Err('日'));
    }

    #[test]
    fn decodifica_com_codificacao_forcada() {
        // 0xE9 = é em Latin-1; 0x80 = € só em Windows-1252.
        let d = decode_as(b"caf\xe9 \x80", Some(Encoding::Windows1252));
        assert_eq!(d.text, "café €");
        assert_eq!(d.encoding, Encoding::Windows1252);
        assert_eq!(
            encode(&d.text, d.eol, d.encoding, d.bom).unwrap(),
            b"caf\xe9 \x80"
        );
        let d = decode_as(b"\xa4", Some(Encoding::Iso885915));
        assert_eq!(d.text, "€");
        assert_eq!(encode("€", Eol::Lf, Encoding::Latin1, false), Err('€'));
    }

    #[test]
    fn utf8_forcado_nao_falha_com_bytes_invalidos() {
        assert_eq!(decode_as(b"a\xe9", Some(Encoding::Utf8)).text, "a\u{FFFD}");
    }

    #[test]
    fn bom_utf8_e_preservado() {
        let d = decode(b"\xEF\xBB\xBFola");
        assert!(d.bom);
        assert_eq!(d.text, "ola");
        assert_eq!(
            encode(&d.text, d.eol, d.encoding, d.bom).unwrap(),
            b"\xEF\xBB\xBFola"
        );
    }
}
