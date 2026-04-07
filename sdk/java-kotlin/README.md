# PO for Java / Kotlin — Protocol Orzatty JVM SDK

Use Protocol Orzatty in JVM applications via UniFFI-generated JNA bindings.

## Requirements

- Java 11+ or Kotlin 1.8+
- The UniFFI-generated `.jar` or Kotlin source files
- `po_ffi.dll` / `libpo_ffi.so` native library

## Installation (Gradle/Kotlin DSL)

```kotlin
dependencies {
    implementation(files("libs/po-ffi.jar"))
}
```

Place the native library in your `src/main/resources/` or system library path.

## Quick Start (Kotlin)

```kotlin
import po.PoClient

fun main() {
    val client = PoClient("0", "127.0.0.1:9091")
    println("Node ID: ${client.nodeId()}")

    client.send("Kotlin E2EE QUIC payload".toByteArray())
    println("Data sent securely via Protocol Orzatty")

    client.close()
}
```

## Quick Start (Java)

```java
import po.PoClient;

public class Demo {
    public static void main(String[] args) {
        PoClient client = new PoClient("0", "127.0.0.1:9091");
        System.out.println("Node ID: " + client.nodeId());

        client.send("Java E2EE payload".getBytes());
        client.close();
    }
}
```

## Android (Kotlin)

The `.so` library can be placed in `jniLibs/` for Android projects:

```
app/
  src/main/
    jniLibs/
      arm64-v8a/libpo_ffi.so
      x86_64/libpo_ffi.so
```

---

*Built by [Orzatty Corporation](https://orzatty.com)*
