//! Leitura e escrita de arquivos de texto com detecção de codificação e fim de linha (TL-301, ADR 26).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Eol {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Utf8,
    /// Fallback quando os bytes não são UTF-8 válido (o editor avisa o usuário).
    Latin1,
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
    let (body, bom) = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => (rest, true),
        None => (bytes, false),
    };
    let (raw, encoding) = match std::str::from_utf8(body) {
        Ok(s) => (s.to_owned(), Encoding::Utf8),
        Err(_) => (body.iter().map(|&b| b as char).collect(), Encoding::Latin1),
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

/// Codifica de volta. Falha com o primeiro caractere não representável em Latin-1.
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
        Encoding::Latin1 => {
            for c in body.chars() {
                out.push(u8::try_from(u32::from(c)).map_err(|_| c)?);
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
