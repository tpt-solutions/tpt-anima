//! eBPF kernel module for Linux workload interception.
//!
//! This module provides the eBPF (extended Berkeley Packet Filter) program definitions
//! for intercepting AI workloads at the kernel level. The actual eBPF bytecode is
//! generated and loaded at runtime.

use serde::{Deserialize, Serialize};

/// An eBPF program definition for workload interception.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbpfProgram {
    /// Program name/identifier.
    pub name: String,
    /// eBPF bytecode (compiled from restricted C/Rust).
    pub bytecode: Vec<u8>,
    /// Map definitions for the program.
    pub maps: Vec<EbpfMap>,
}

/// An eBPF map definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbpfMap {
    /// Map name.
    pub name: String,
    /// Map type (hash, array, etc.).
    pub map_type: EbpfMapType,
    /// Key size in bytes.
    pub key_size: u32,
    /// Value size in bytes.
    pub value_size: u32,
}

/// eBPF map types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EbpfMapType {
    /// Hash map.
    Hash,
    /// Array map.
    Array,
    /// Per-CPU array.
    PerCpuArray,
    /// Queue.
    Queue,
    /// Stack.
    Stack,
}

/// Hook points where eBPF programs can attach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookPoint {
    /// Tracepoint for syscall entry.
    SyscallEntry,
    /// Tracepoint for syscall exit.
    SyscallExit,
    /// Kprobe (kernel function entry).
    Kprobe,
    /// Kretprobe (kernel function return).
    Kretprobe,
    /// Uprobe (user function entry).
    Uprobe,
    /// Network interface receive.
    NetDevQueue,
}

impl EbpfProgram {
    /// Generate stub eBPF bytecode for AI workload interception.
    /// 
    /// In a full implementation, this would compile from eBPF source (C or Rust).
    pub fn generate_inference_intercept() -> Self {
        Self {
            name: "anima_inference_intercept".to_string(),
            bytecode: vec![
                // Stub eBPF bytecode - in production this would be the compiled program
                0xb7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov r0, 0
            ],
            maps: vec![
                EbpfMap {
                    name: "intercept_events".to_string(),
                    map_type: EbpfMapType::Hash,
                    key_size: 8,
                    value_size: 64,
                },
            ],
        }
    }

    /// Generate eBPF bytecode for memory tracing.
    pub fn generate_memory_trace() -> Self {
        Self {
            name: "anima_memory_trace".to_string(),
            bytecode: vec![
                // Stub eBPF bytecode
            ],
            maps: vec![EbpfMap {
                name: "memory_events".to_string(),
                map_type: EbpfMapType::PerCpuArray,
                key_size: 4,
                value_size: 128,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebpf_program_generates_inference() {
        let prog = EbpfProgram::generate_inference_intercept();
        assert_eq!(prog.name, "anima_inference_intercept");
        assert!(!prog.maps.is_empty());
    }

    #[test]
    fn ebpf_program_memory_trace() {
        let prog = EbpfProgram::generate_memory_trace();
        assert_eq!(prog.name, "anima_memory_trace");
        assert_eq!(prog.maps[0].map_type, EbpfMapType::PerCpuArray);
    }
}