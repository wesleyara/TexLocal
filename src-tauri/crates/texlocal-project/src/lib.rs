//! Camada de projeto do editor (EP03): arquivos de texto, índice multi-arquivo,
//! busca, contagem de palavras e ortografia.

pub mod index;
pub mod prose;
pub mod search;
pub mod spell;
pub mod textfile;
pub mod wordcount;

/// Converte um deslocamento em bytes de `line` para unidades UTF-16 (as do CodeMirror).
pub(crate) fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}
