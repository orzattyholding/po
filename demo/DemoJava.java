// Requires uniffi generated JNA or React Native Native Modules bindings
import po.PoClient;

public class DemoJava {
    public static void main(String[] args) {
        System.out.println("Starting Java JVM / Kotlin PO Protocol Client");
        
        try {
            PoClient client = new PoClient("0", "127.0.0.1:9091");
            System.out.println("Node ID: " + client.nodeId());
            
            client.send("Java Native JNA Mission".getBytes());
            System.out.println("Data routed securely over PO.");
            
            client.close();
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
