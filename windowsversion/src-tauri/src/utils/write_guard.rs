//! 项目级写通道串行锁（ADR-003）
//!
//! 每个项目独立 `Mutex<()>`；同 project_id 的 5 个写命令串行，
//! 不同 project 互不阻塞；read & 缩略图不取锁。
//!
//! 用法（T3 命令首行）：
//! ```ignore
//! let lock = guard.lock_for(&project_id);
//! let _g = lock.lock().expect("锁中毒");
//! // ... 写操作 ...
//! // _g drop 时释放
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 双层结构：外层 map 锁短暂持有（仅找/创内层锁），内层锁长期持有（覆盖整个写命令）。
pub struct WorkspaceWriteGuard {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl WorkspaceWriteGuard {
    pub fn new() -> Self {
        Self { locks: Mutex::new(HashMap::new()) }
    }

    /// 返回 project 的写锁 `Arc<Mutex<()>>`；调用方需自行 `.lock()` 持有 guard。
    /// 拆成两步是为了避免 OwnedMutexGuard 复杂的生命周期/unsafe；
    /// 调用方写法见模块顶部示例。
    pub fn lock_for(&self, project_id: &str) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().expect("WorkspaceWriteGuard outer 锁中毒");
        map.entry(project_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

impl Default for WorkspaceWriteGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    /// 同 project_id 并发：两线程必须串行执行，总耗时 ≥ 2 * sleep。
    #[test]
    fn same_project_serializes() {
        let g = Arc::new(WorkspaceWriteGuard::new());
        let counter = Arc::new(AtomicU32::new(0));
        let max_concurrent = Arc::new(AtomicU32::new(0));

        let start = Instant::now();
        let mut handles = vec![];
        for _ in 0..2 {
            let g = g.clone();
            let counter = counter.clone();
            let mc = max_concurrent.clone();
            handles.push(thread::spawn(move || {
                let lock = g.lock_for("p1");
                let _guard = lock.lock().unwrap();
                let now = counter.fetch_add(1, Ordering::SeqCst) + 1;
                mc.fetch_max(now, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(80));
                counter.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed();
        assert_eq!(max_concurrent.load(Ordering::SeqCst), 1, "同 project 必须串行（并发上限=1）");
        assert!(elapsed >= Duration::from_millis(160), "总耗时应 >= 2 * 80ms，实际 {:?}", elapsed);
    }

    /// 不同 project_id 并发：两线程应并行（总耗时接近单次 sleep）。
    #[test]
    fn different_projects_parallel() {
        let g = Arc::new(WorkspaceWriteGuard::new());
        let max_concurrent = Arc::new(AtomicU32::new(0));
        let counter = Arc::new(AtomicU32::new(0));

        let start = Instant::now();
        let mut handles = vec![];
        for pid in ["p1", "p2"] {
            let g = g.clone();
            let mc = max_concurrent.clone();
            let counter = counter.clone();
            let pid = pid.to_string();
            handles.push(thread::spawn(move || {
                let lock = g.lock_for(&pid);
                let _guard = lock.lock().unwrap();
                let now = counter.fetch_add(1, Ordering::SeqCst) + 1;
                mc.fetch_max(now, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(120));
                counter.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let elapsed = start.elapsed();
        assert_eq!(max_concurrent.load(Ordering::SeqCst), 2, "不同 project 应并行（并发上限=2）");
        assert!(elapsed < Duration::from_millis(220), "应小于 2*120ms（并行），实际 {:?}", elapsed);
    }

    /// 同 project 重复 lock_for 应返回相同 Arc（同一锁对象）。
    #[test]
    fn lock_for_returns_same_arc_for_same_project() {
        let g = WorkspaceWriteGuard::new();
        let a = g.lock_for("p1");
        let b = g.lock_for("p1");
        assert!(Arc::ptr_eq(&a, &b), "同 project_id 应共享同一 Arc<Mutex>");

        let c = g.lock_for("p2");
        assert!(!Arc::ptr_eq(&a, &c), "不同 project_id 应不同 Arc");
    }
}
