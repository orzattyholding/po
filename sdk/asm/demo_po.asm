; ══════════════════════════════════════════════════════════════════
; Protocol Orzatty — x86_64 Assembly SDK Demo (NASM, Windows)
;
; The most bare-metal E2EE networking protocol on Earth.
; This file calls the PO C-ABI functions directly from Assembly.
;
; Build:
;   nasm -f win64 demo_po.asm -o demo_po.obj
;   link demo_po.obj po_c.lib /subsystem:console /entry:main
;
; Requires: po_c.lib + po.h (from the PO C SDK release)
; ══════════════════════════════════════════════════════════════════

bits 64
default rel

; ── External imports from po_c.lib ──────────────────────────
extern po_client_new
extern po_client_send
extern po_client_free

; ── C runtime for console output ────────────────────────────
extern printf
extern ExitProcess

section .data
    ; Connection parameters
    bind_addr:      db "0", 0
    remote_addr:    db "127.0.0.1:9091", 0

    ; Payload — every byte of this will be encrypted with XChaCha20-Poly1305
    payload:        db "x86_64 ASM over E2EE QUIC — Orzatty Protocol", 0
    payload_len:    equ $ - payload - 1  ; exclude null terminator

    ; Console messages
    msg_start:      db "[ASM] Protocol Orzatty x86_64 SDK initializing...", 10, 0
    msg_connected:  db "[ASM] E2EE handshake complete. Client handle: %p", 10, 0
    msg_sent:       db "[ASM] Encrypted payload sent (%d bytes) via QUIC/UDP", 10, 0
    msg_fail_conn:  db "[ASM] FATAL: E2EE handshake failed (NULL handle)", 10, 0
    msg_fail_send:  db "[ASM] FATAL: Send returned error code %d", 10, 0
    msg_freed:      db "[ASM] Resources released. Clean exit.", 10, 0

section .bss
    client_handle:  resq 1   ; Pointer to PoClientC*

section .text
global main

main:
    ; ── Prologue ────────────────────────────────────────────
    push    rbp
    mov     rbp, rsp
    sub     rsp, 64         ; Shadow space + alignment (Windows x64 ABI)

    ; ── Print banner ────────────────────────────────────────
    lea     rcx, [msg_start]
    call    printf

    ; ── po_client_new(bind_addr, remote_addr) ───────────────
    ; Windows x64 calling convention: rcx = arg1, rdx = arg2
    lea     rcx, [bind_addr]
    lea     rdx, [remote_addr]
    call    po_client_new

    ; Check for NULL (handshake failure)
    test    rax, rax
    jz      .connection_failed

    ; Store the client handle
    mov     [client_handle], rax

    ; Print success
    lea     rcx, [msg_connected]
    mov     rdx, rax
    call    printf

    ; ── po_client_send(client, data, len) ───────────────────
    ; rcx = client handle
    ; rdx = pointer to payload data
    ; r8  = length of payload
    mov     rcx, [client_handle]
    lea     rdx, [payload]
    mov     r8, payload_len
    call    po_client_send

    ; Check return (0 = success)
    test    eax, eax
    jnz     .send_failed

    ; Print success
    lea     rcx, [msg_sent]
    mov     edx, payload_len
    call    printf

    ; ── po_client_free(client) ──────────────────────────────
    mov     rcx, [client_handle]
    call    po_client_free

    ; Print cleanup
    lea     rcx, [msg_freed]
    call    printf

    ; ── Exit ────────────────────────────────────────────────
    xor     ecx, ecx        ; exit code 0
    call    ExitProcess

.connection_failed:
    lea     rcx, [msg_fail_conn]
    call    printf
    mov     ecx, 1
    call    ExitProcess

.send_failed:
    lea     rcx, [msg_fail_send]
    mov     edx, eax
    call    printf
    ; Free the client even on error
    mov     rcx, [client_handle]
    call    po_client_free
    mov     ecx, 2
    call    ExitProcess
