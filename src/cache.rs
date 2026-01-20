// game-translator/src/cache.rs

// ============================================================================
// MÓDULO CACHE - Cache de traduções para evitar chamadas repetidas à API
// ============================================================================
//
// O cache guarda traduções já feitas em memória e opcionalmente em disco.
// Isso acelera muito quando o mesmo texto aparece várias vezes (ex: legendas).
//
// ============================================================================

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Estrutura do cache de traduções
#[derive(Debug, Clone)]
pub struct TranslationCache {
    /// HashMap: chave = "provider:source_lang:target_lang:texto_original"
    /// valor = texto traduzido
    cache: Arc<Mutex<HashMap<String, String>>>,
    /// Caminho do arquivo de cache (para persistência)
    cache_file: String,
    /// Se true, salva cache em disco automaticamente
    persist_to_disk: bool,
}

impl TranslationCache {
    /// Cria um novo cache
    pub fn new(persist_to_disk: bool) -> Self {
        let cache_file = "translation_cache.json".to_string();
        let mut cache = TranslationCache {
            cache: Arc::new(Mutex::new(HashMap::new())),
            cache_file,
            persist_to_disk,
        };

        // Tenta carregar cache existente do disco
        if persist_to_disk {
            if let Err(e) = cache.load_from_disk() {
                info!("📦 Cache vazio ou não encontrado: {}", e);
            }
        }

        cache
    }

    /// Gera a chave única para um texto
    fn make_key(provider: &str, source_lang: &str, target_lang: &str, text: &str) -> String {
        format!("{}:{}:{}:{}", provider, source_lang, target_lang, text)
    }

    /// Busca uma tradução no cache
    pub fn get(
        &self,
        provider: &str,
        source_lang: &str,
        target_lang: &str,
        text: &str,
    ) -> Option<String> {
        let key = Self::make_key(provider, source_lang, target_lang, text);
        let cache = self.cache.lock().unwrap();
        cache.get(&key).cloned()
    }

    /// Adiciona uma tradução ao cache
    pub fn set(
        &self,
        provider: &str,
        source_lang: &str,
        target_lang: &str,
        original: &str,
        translated: &str,
    ) {
        let key = Self::make_key(provider, source_lang, target_lang, original);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key, translated.to_string());
    }

    /// Busca múltiplas traduções no cache
    /// Retorna (encontrados, não_encontrados)
    pub fn get_batch(
        &self,
        provider: &str,
        source_lang: &str,
        target_lang: &str,
        texts: &[String],
    ) -> (Vec<(usize, String)>, Vec<(usize, String)>) {
        let cache = self.cache.lock().unwrap();
        let mut found: Vec<(usize, String)> = Vec::new();
        let mut not_found: Vec<(usize, String)> = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            let key = Self::make_key(provider, source_lang, target_lang, text);
            if let Some(translated) = cache.get(&key) {
                found.push((i, translated.clone()));
            } else {
                not_found.push((i, text.clone()));
            }
        }

        (found, not_found)
    }

    /// Adiciona múltiplas traduções ao cache
    pub fn set_batch(
        &self,
        provider: &str,
        source_lang: &str,
        target_lang: &str,
        pairs: &[(String, String)], // (original, translated)
    ) {
        let mut cache = self.cache.lock().unwrap();
        for (original, translated) in pairs {
            let key = Self::make_key(provider, source_lang, target_lang, original);
            cache.insert(key, translated.clone());
        }
    }

    /// Retorna estatísticas do cache
    pub fn stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        let total = cache.len();
        let size_bytes: usize = cache.iter().map(|(k, v)| k.len() + v.len()).sum();
        (total, size_bytes)
    }

    /// Limpa o cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        info!("🗑️  Cache limpo!");
    }

    /// Salva o cache em disco
    pub fn save_to_disk(&self) -> Result<()> {
        if !self.persist_to_disk {
            return Ok(());
        }

        let cache = self.cache.lock().unwrap();
        let json = serde_json::to_string_pretty(&*cache).context("Falha ao serializar cache")?;

        fs::write(&self.cache_file, json).context("Falha ao salvar cache em disco")?;

        info!("💾 Cache salvo: {} entradas", cache.len());
        Ok(())
    }

    /// Carrega o cache do disco
    pub fn load_from_disk(&mut self) -> Result<()> {
        if !Path::new(&self.cache_file).exists() {
            return Ok(());
        }

        let json = fs::read_to_string(&self.cache_file).context("Falha ao ler arquivo de cache")?;

        let loaded: HashMap<String, String> =
            serde_json::from_str(&json).context("Falha ao parsear cache")?;

        let mut cache = self.cache.lock().unwrap();
        *cache = loaded;

        info!("📦 Cache carregado: {} entradas", cache.len());
        Ok(())
    }
}

impl Drop for TranslationCache {
    fn drop(&mut self) {
        // Salva o cache quando o programa encerra
        if self.persist_to_disk {
            let _ = self.save_to_disk();
        }
    }
}
