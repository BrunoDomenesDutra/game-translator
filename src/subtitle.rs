// game-translator/src/subtitle.rs

// ============================================================================
// MÓDULO SUBTITLE - Sistema de legendas em tempo real (modo histórico)
// ============================================================================
//
// Este módulo gerencia a captura contínua de legendas, detectando quando
// o texto muda e mantendo um histórico de traduções para exibição.
//
// Melhorias implementadas:
// - Levenshtein Distance: comparação mais precisa entre textos
// - Debounce/Estabilização: só aceita legenda se permanecer estável
//
// ============================================================================

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Número máximo de legendas no histórico
const MAX_SUBTITLE_HISTORY: usize = 10;

/// Threshold de similaridade Levenshtein (0.0 a 1.0)
/// Textos com similaridade acima deste valor são considerados "iguais"
const LEVENSHTEIN_SIMILARITY_THRESHOLD: f64 = 0.75;

/// Número mínimo de caracteres para considerar um texto válido
const MIN_TEXT_LENGTH: usize = 3;

/// Representa uma legenda traduzida no histórico
#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    /// Texto traduzido
    pub translated: String,
    /// Momento em que foi adicionada
    pub added_at: Instant,
}

/// Estado do candidato a legenda (para debounce)
#[derive(Debug, Clone)]
struct SubtitleCandidate {
    /// Texto detectado
    text: String,
    /// Quando foi detectado pela primeira vez
    first_seen: Instant,
    /// Quantas vezes foi visto consecutivamente
    seen_count: u32,
}

/// Estado do sistema de legendas
#[derive(Clone)]
pub struct SubtitleState {
    /// Último texto confirmado (já traduzido)
    last_confirmed_text: Arc<Mutex<String>>,
    /// Candidato atual (aguardando estabilização)
    current_candidate: Arc<Mutex<Option<SubtitleCandidate>>>,
    /// Histórico de legendas traduzidas
    subtitle_history: Arc<Mutex<Vec<SubtitleEntry>>>,
    /// Número de vezes que o texto precisa ser visto para confirmar (debounce)
    required_stable_count: u32,
}

impl SubtitleState {
    /// Cria um novo estado de legendas
    ///
    /// # Argumentos
    /// * `_min_display_secs` - Não usado mais (mantido para compatibilidade)
    /// * `_max_display_secs` - Não usado mais (mantido para compatibilidade)
    pub fn new(_min_display_secs: u64, _max_display_secs: u64) -> Self {
        SubtitleState {
            last_confirmed_text: Arc::new(Mutex::new(String::new())),
            current_candidate: Arc::new(Mutex::new(None)),
            subtitle_history: Arc::new(Mutex::new(Vec::new())),
            // Requer 2 detecções consecutivas para confirmar
            // Com intervalo de 500ms, isso significa ~1 segundo de estabilidade
            required_stable_count: 2,
        }
    }

    /// Processa um novo texto detectado pelo OCR
    ///
    /// Usa sistema de debounce: só retorna texto para tradução quando
    /// o mesmo texto é detectado múltiplas vezes consecutivas.
    ///
    /// # Retorna
    /// * `Some(texto)` - Se o texto foi confirmado e precisa ser traduzido
    /// * `None` - Se o texto é igual ao anterior, muito curto, ou ainda não estabilizou
    pub fn process_detected_text(&self, new_text: &str) -> Option<String> {
        // Normaliza o texto
        let normalized_new = normalize_text(new_text);

        // Se o texto está vazio ou muito curto, ignora
        if normalized_new.len() < MIN_TEXT_LENGTH {
            return None;
        }

        // Verifica se é similar ao último texto confirmado
        let last_confirmed = self.last_confirmed_text.lock().unwrap();
        let normalized_last = normalize_text(&last_confirmed);

        if texts_are_similar_levenshtein(&normalized_new, &normalized_last) {
            // Texto é igual ou muito similar ao último confirmado, ignora
            return None;
        }
        drop(last_confirmed); // Libera o lock

        // Sistema de debounce: verifica candidato atual
        let mut candidate = self.current_candidate.lock().unwrap();

        match &mut *candidate {
            Some(current) => {
                // Já temos um candidato, verifica se é o mesmo texto
                let normalized_candidate = normalize_text(&current.text);

                if texts_are_similar_levenshtein(&normalized_new, &normalized_candidate) {
                    // Mesmo texto! Incrementa contador
                    current.seen_count += 1;

                    if current.seen_count >= self.required_stable_count {
                        // Texto estabilizou! Confirma e retorna para tradução
                        let confirmed_text = current.text.clone();

                        // Atualiza último texto confirmado
                        *self.last_confirmed_text.lock().unwrap() = confirmed_text.clone();

                        // Limpa candidato
                        *candidate = None;

                        info!(
                            "📺 Legenda confirmada após {} detecções: \"{}\"",
                            self.required_stable_count, confirmed_text
                        );

                        return Some(confirmed_text);
                    } else {
                        // Ainda não estabilizou
                        trace!(
                            "📺 Candidato visto {}/{} vezes",
                            current.seen_count,
                            self.required_stable_count
                        );
                        return None;
                    }
                } else {
                    // Texto diferente! Substitui candidato
                    info!("📺 Novo candidato detectado: \"{}\"", new_text.trim());
                    *candidate = Some(SubtitleCandidate {
                        text: new_text.trim().to_string(),
                        first_seen: Instant::now(),
                        seen_count: 1,
                    });
                    return None;
                }
            }
            None => {
                // Não temos candidato, cria um novo
                info!("📺 Primeiro candidato detectado: \"{}\"", new_text.trim());
                *candidate = Some(SubtitleCandidate {
                    text: new_text.trim().to_string(),
                    first_seen: Instant::now(),
                    seen_count: 1,
                });
                return None;
            }
        }
    }

    /// Adiciona uma legenda traduzida ao histórico
    pub fn add_translated_subtitle(&self, translated: String) {
        let mut history = self.subtitle_history.lock().unwrap();

        // Adiciona a nova legenda
        history.push(SubtitleEntry {
            translated,
            added_at: Instant::now(),
        });

        // Remove legendas antigas se exceder o limite
        while history.len() > MAX_SUBTITLE_HISTORY {
            history.remove(0);
        }

        info!("📺 Histórico de legendas: {} itens", history.len());
    }

    /// Obtém o histórico de legendas para exibição
    pub fn get_subtitle_history(&self) -> Vec<SubtitleEntry> {
        let history = self.subtitle_history.lock().unwrap();
        history.clone()
    }

    /// Verifica se há legendas para exibir
    pub fn has_subtitles(&self) -> bool {
        let history = self.subtitle_history.lock().unwrap();
        !history.is_empty()
    }

    /// Limpa o histórico (quando desativa o modo legenda)
    pub fn clear(&self) {
        *self.last_confirmed_text.lock().unwrap() = String::new();
        *self.current_candidate.lock().unwrap() = None;
        self.subtitle_history.lock().unwrap().clear();
        info!("📺 Histórico de legendas limpo");
    }
}

// ============================================================================
// FUNÇÕES AUXILIARES
// ============================================================================

/// Normaliza texto para comparação
/// Remove espaços extras, converte para minúsculas, remove caracteres especiais
fn normalize_text(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
}

/// Calcula a distância de Levenshtein entre duas strings
///
/// A distância de Levenshtein é o número mínimo de edições (inserções,
/// remoções ou substituições) necessárias para transformar uma string em outra.
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    // Casos especiais
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    // Matriz de distâncias (otimizada para usar apenas 2 linhas)
    let mut prev_row: Vec<usize> = (0..=len2).collect();
    let mut curr_row: Vec<usize> = vec![0; len2 + 1];

    for i in 1..=len1 {
        curr_row[0] = i;

        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };

            curr_row[j] = (prev_row[j] + 1) // Remoção
                .min(curr_row[j - 1] + 1) // Inserção
                .min(prev_row[j - 1] + cost); // Substituição
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[len2]
}

/// Calcula a similaridade entre duas strings usando Levenshtein
///
/// # Retorna
/// Valor entre 0.0 (totalmente diferentes) e 1.0 (idênticas)
fn levenshtein_similarity(s1: &str, s2: &str) -> f64 {
    let max_len = s1.len().max(s2.len());

    if max_len == 0 {
        return 1.0; // Ambas vazias = idênticas
    }

    let distance = levenshtein_distance(s1, s2);
    1.0 - (distance as f64 / max_len as f64)
}

/// Verifica se dois textos são similares usando Levenshtein Distance
fn texts_are_similar_levenshtein(text1: &str, text2: &str) -> bool {
    // Se um está vazio e outro não, são diferentes
    if text1.is_empty() != text2.is_empty() {
        return false;
    }

    // Se ambos vazios, são iguais
    if text1.is_empty() && text2.is_empty() {
        return true;
    }

    let similarity = levenshtein_similarity(text1, text2);

    // Log para debug
    if similarity > 0.5 && similarity < LEVENSHTEIN_SIMILARITY_THRESHOLD {
        trace!(
            "📊 Similaridade: {:.2}% entre \"{}\" e \"{}\"",
            similarity * 100.0,
            text1,
            text2
        );
    }

    similarity >= LEVENSHTEIN_SIMILARITY_THRESHOLD
}

/// Verifica se dois textos são similares (método legado com HashSet)
/// Mantido para referência, mas não usado
#[allow(dead_code)]
fn texts_are_similar_charset(text1: &str, text2: &str) -> bool {
    if text1.is_empty() != text2.is_empty() {
        return false;
    }

    if text1.is_empty() && text2.is_empty() {
        return true;
    }

    let len1 = text1.len();
    let len2 = text2.len();
    let len_diff = (len1 as i32 - len2 as i32).abs() as usize;
    let max_len = len1.max(len2);

    if max_len > 0 && len_diff > max_len / 5 {
        return false;
    }

    let chars1: HashSet<char> = text1.chars().collect();
    let chars2: HashSet<char> = text2.chars().collect();
    let common = chars1.intersection(&chars2).count();
    let total = chars1.union(&chars2).count();

    if total == 0 {
        return true;
    }

    (common as f64 / total as f64) > 0.85
}
