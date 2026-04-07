# PO for Flutter / Dart — Protocol Orzatty Mobile SDK

Use Protocol Orzatty in Flutter and Dart applications via `dart:ffi`.

## Requirements

- Flutter 3.0+ / Dart 2.18+
- `po_c.dll` (Windows) / `libpo_c.so` (Linux/Android) / `libpo_c.dylib` (macOS/iOS)
- `po.h` header for reference

## Installation

### Flutter (pubspec.yaml)

```yaml
# Place the native library in your platform-specific folders:
# android/app/src/main/jniLibs/arm64-v8a/libpo_c.so
# ios/Frameworks/libpo_c.dylib
# windows/po_c.dll
# linux/libpo_c.so
```

## Quick Start

```dart
import 'dart:ffi';
import 'dart:io' show Platform;

// Load the native library
final DynamicLibrary poLib = Platform.isWindows
    ? DynamicLibrary.open('po_c.dll')
    : DynamicLibrary.open('libpo_c.so');

// Bind native functions
typedef PoClientNewNative = Pointer<Void> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef PoClientNew = Pointer<Void> Function(Pointer<Utf8>, Pointer<Utf8>);

typedef PoClientSendNative = Int32 Function(Pointer<Void>, Pointer<Uint8>, IntPtr);
typedef PoClientSend = int Function(Pointer<Void>, Pointer<Uint8>, int);

typedef PoClientFreeNative = Void Function(Pointer<Void>);
typedef PoClientFree = void Function(Pointer<Void>);

final poClientNew = poLib.lookupFunction<PoClientNewNative, PoClientNew>('po_client_new');
final poClientSend = poLib.lookupFunction<PoClientSendNative, PoClientSend>('po_client_send');
final poClientFree = poLib.lookupFunction<PoClientFreeNative, PoClientFree>('po_client_free');

void main() {
  final bind = '0'.toNativeUtf8();
  final remote = '127.0.0.1:9091'.toNativeUtf8();

  final client = poClientNew(bind, remote);
  print('PO client connected via E2EE QUIC');

  // Send data
  final msg = 'Flutter E2EE payload'.toNativeUtf8();
  poClientSend(client, msg.cast(), 20);

  poClientFree(client);
}
```

## Architecture

Flutter → `dart:ffi` → `po_c.dll`/`.so` → Rust Core (QUIC + E2EE)

Zero abstraction layers. Native speed on mobile.

---

*Built by [Orzatty Corporation](https://orzatty.com)*
