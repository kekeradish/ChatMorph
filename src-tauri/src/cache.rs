use std::collections::{HashMap, VecDeque};

/// 命中率极高的场景：游戏报点、常用语几乎是同一批句子。
/// 命中直接返回，省掉 1.4 秒的模型请求。
const MAX_ENTRIES: usize = 200;

pub struct TranslationCache {
    map: HashMap<String, String>,
    /// 记录访问顺序，队首是最久未用的
    order: VecDeque<String>,
}

impl TranslationCache {
    pub fn new() -> Self {
        Self {
            map: HashMap::with_capacity(MAX_ENTRIES),
            order: VecDeque::with_capacity(MAX_ENTRIES),
        }
    }

    /// 命中则把它移到队尾（最近使用）
    pub fn get(&mut self, key: &str) -> Option<String> {
        let value = self.map.get(key).cloned();
        if value.is_some() {
            self.order.retain(|k| k != key);
            self.order.push_back(key.to_string());
        }
        value
    }

    pub fn put(&mut self, key: String, value: String) {
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else if self.map.len() >= MAX_ENTRIES {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_oldest_when_full() {
        let mut c = TranslationCache::new();
        for i in 0..MAX_ENTRIES {
            c.put(format!("k{i}"), format!("v{i}"));
        }
        // 访问一下 k0，让它变成"最近使用"
        assert_eq!(c.get("k0"), Some("v0".to_string()));
        c.put("new".to_string(), "vnew".to_string());
        // k0 被救活，k1 才是被淘汰的那个
        assert_eq!(c.get("k0"), Some("v0".to_string()));
        assert_eq!(c.get("k1"), None);
        assert_eq!(c.get("new"), Some("vnew".to_string()));
        assert_eq!(c.len(), MAX_ENTRIES);
    }
}
