## 实现的功能

实现了系统调用 `sys_spawn`。
实现了系统调用 `sys_set_priority`。 

## 完成问答题
### 实际情况是轮到 p1 执行吗？为什么？

不会。

因为使用 8bit 无符号整形储存 stride，p2.stride 会在加 10 以后溢出，变得比 p1.stride 小。

### 为什么？尝试简单说明（不要求严格证明）。

无论何时，如何 STRIDE_MAX – STRIDE_MIN <= BigStride / 2 不成立，说明调度已经出了问题，BigStride / 2 是 pass 的最大值，在调度没有出现问题的情况下，不可能出现 STRIDE_MAX – STRIDE_MIN > BigStride / 2。

### 补全代码
```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.0.cmp(&other.0) {
            Ordering::Less => {
                if other.0 - self.0 > BIG_STRIDE / MIN_PRIORITY as u64 {
                    Some(Ordering::Greater)
                } else {
                    Some(Ordering::Less)
                }
            }
            Ordering::Greater => {
                if self.0 - other.0 > BIG_STRIDE / MIN_PRIORITY as u64 {
                    Some(Ordering::Less)
                } else {
                    Some(Ordering::Greater)
                }
            }
            Ordering::Equal => Some(Ordering::Equal),
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

## 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

没有交流的对象。

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

没有参考的资料。

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。