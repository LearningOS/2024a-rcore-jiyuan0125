## 实现的功能

实现了系统调用 `sys_task_info`。
步骤如下：
1. 在 `TaskControlBlock` 结构体中添加了 `syscall_times`, `start_time` 字段。
2. 在 syscall 分发处理前更新 `syscall_times`。
3. 在 `TaskManager` 的 `run_next_task` 函数中初始化 `start_time`。
4. `sys_task_info` 函数里获取当前任务的 `TaskControlBlock`，并计算运行时间，构建 `TaskInfo`。

## 完成问答题
### 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

`ch2b_bad_address.rs` 尝试对 `0x0` 地址进行写操作，会触发异常 Trap::Exception(Exception::StoreFault)。

`ch2b_bad_instructions.rs` 尝试执行 `sret` 指令，这在 U 态是不允许的，会触发异常 Trap::Exception(Exception::IllegalInstruction)。

`ch2b_bad_register.rs` 尝试访问 S 态寄存器 `sstatus`，这在 U 态是不允许的，会触发异常 Trap::Exception(Exception::IllegalInstruction)。

sbi 版本信息： RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0

### 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:
#### L40：刚进入 __restore 时，a0 代表了什么值。请指出 __restore 的两种使用情景。
`a0` 是 `current_task_cx_ptr: *mut TaskContext`, 即上一个任务的上下文指针。

`__restore` 有两种使用情景：
1. 首次运行任务时
2. 任务切换时

#### L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的值对于进入用户态有何意义？请分别解释。

特殊处理了 sstatus, sepc, sscratch 寄存器。

sstatus寄存器用于控制CPU的运行状态，包括特权级模式、中断使能等。当发生trap时，CPU需要切换到管理模式（S模式）来处理这个异常或中断。在这个过程中，sstatus寄存器的值会被改变，以反映新的特权级和中断状态。为了在处理完trap后能够恢复到原来的状态，内核需要保存当前的sstatus值，并在处理完毕后将其还原。

sepc寄存器用于保存程序计数器（PC）的值，当发生trap时，CPU会将当前PC值复制到sepc中，以便在处理完trap后能够从原来的位置继续执行。这是为了确保在处理trap期间，原始代码的执行不会丢失或被覆盖。

sscratch寄存器通常用于临时存储一些数据（在这里是用于切换内核态和用户态sp的值），这些信息对于内核来说非常重要，因为它们帮助内核确定如何恢复用户进程的状态。因此，内核也需要在处理trap时保存sscratch寄存器的值，并在处理完毕后将其还原。

#### L50-L56：为何跳过了 x2 和 x4？

跳过 x2（sp）是因为此时还需要 sp 指向内核栈。
跳过 x4 是因为程序中没有使用到 x4。在 __alltraps 中 x4 也没有保存过，所以不需要恢复。

#### L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

sp 指向用户态栈，sscratch 保存了内核态的栈指针。

#### __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

`sret` 指令。`sret` 指令会将 sepc 中的值赋给 pc 寄存器，并根据 sstatus 中的 SPP 记录的特权级别切换到用户态。

#### L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

sp 指向内核态栈，sscratch 保存了用户态的栈指针。

#### 从 U 态进入 S 态是哪一条指令发生的？

`ecall` 指令。

## 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

没有交流的对象。

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

没有参考的资料。

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。