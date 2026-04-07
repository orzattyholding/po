/// Protocol Orzatty (PO) — Flutter/Dart FFI bindings.
///
/// Provides P2P End-to-End Encrypted networking over QUIC.
/// All data is encrypted with XChaCha20-Poly1305 before hitting the wire.
library protocol_orzatty;

import 'dart:ffi';
import 'dart:io' show Platform;

/// Load the native PO library based on the current platform.
DynamicLibrary _loadPoLibrary() {
  if (Platform.isWindows) {
    return DynamicLibrary.open('po_c.dll');
  } else if (Platform.isLinux || Platform.isAndroid) {
    return DynamicLibrary.open('libpo_c.so');
  } else if (Platform.isMacOS || Platform.isIOS) {
    return DynamicLibrary.open('libpo_c.dylib');
  }
  throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
}

// ── Native function typedefs ──────────────────────────────────────

// po_client_new(bind: *const c_char, remote: *const c_char) -> *mut PoClientC
typedef _PoClientNewNative = Pointer<Void> Function(
    Pointer<Utf8>, Pointer<Utf8>);
typedef _PoClientNew = Pointer<Void> Function(Pointer<Utf8>, Pointer<Utf8>);

// po_client_send(client: *mut PoClientC, data: *const u8, len: usize) -> i32
typedef _PoClientSendNative = Int32 Function(
    Pointer<Void>, Pointer<Uint8>, IntPtr);
typedef _PoClientSend = int Function(Pointer<Void>, Pointer<Uint8>, int);

// po_client_free(client: *mut PoClientC) -> void
typedef _PoClientFreeNative = Void Function(Pointer<Void>);
typedef _PoClientFree = void Function(Pointer<Void>);

/// A client for Protocol Orzatty P2P E2EE connections.
class PoClient {
  static final DynamicLibrary _lib = _loadPoLibrary();

  static final _PoClientNew _clientNew =
      _lib.lookupFunction<_PoClientNewNative, _PoClientNew>('po_client_new');
  static final _PoClientSend _clientSend =
      _lib.lookupFunction<_PoClientSendNative, _PoClientSend>('po_client_send');
  static final _PoClientFree _clientFree =
      _lib.lookupFunction<_PoClientFreeNative, _PoClientFree>('po_client_free');

  final Pointer<Void> _handle;
  bool _closed = false;

  PoClient._(this._handle);

  /// Connect to a remote PO node.
  ///
  /// ```dart
  /// final client = PoClient.connect('127.0.0.1:9091');
  /// ```
  factory PoClient.connect(String remoteAddress) {
    final bind = '0'.toNativeUtf8();
    final remote = remoteAddress.toNativeUtf8();
    final handle = _clientNew(bind, remote);
    calloc.free(bind);
    calloc.free(remote);

    if (handle == nullptr) {
      throw StateError('E2EE handshake failed — could not connect to $remoteAddress');
    }
    return PoClient._(handle);
  }

  /// Bind as a server on the given port.
  ///
  /// ```dart
  /// final server = PoClient.bind(4433);
  /// ```
  factory PoClient.bind(int port) {
    final bind = port.toString().toNativeUtf8();
    final handle = _clientNew(bind, nullptr);
    calloc.free(bind);

    if (handle == nullptr) {
      throw StateError('Failed to bind on port $port');
    }
    return PoClient._(handle);
  }

  /// Send encrypted data to the connected peer.
  ///
  /// Returns `true` on success, `false` on error.
  bool send(List<int> data) {
    if (_closed) throw StateError('Client is closed');

    final ptr = calloc<Uint8>(data.length);
    for (var i = 0; i < data.length; i++) {
      ptr[i] = data[i];
    }
    final result = _clientSend(_handle, ptr, data.length);
    calloc.free(ptr);
    return result == 0;
  }

  /// Release all resources.
  void close() {
    if (!_closed) {
      _clientFree(_handle);
      _closed = true;
    }
  }
}
