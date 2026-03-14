package com.httparena;

import io.grpc.Server;
import io.grpc.ServerBuilder;
import io.grpc.netty.shaded.io.grpc.netty.GrpcSslContexts;
import io.grpc.netty.shaded.io.grpc.netty.NettyServerBuilder;
import io.grpc.netty.shaded.io.netty.handler.ssl.SslContext;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.context.event.ContextRefreshedEvent;
import org.springframework.context.event.EventListener;

import java.io.File;

@SpringBootApplication
public class Application {

    public static void main(String[] args) {
        SpringApplication app = new SpringApplication(Application.class);
        app.run(args);
    }

    @EventListener(ContextRefreshedEvent.class)
    public void onStartup() throws Exception {
        File certFile = new File("/certs/server.crt");
        File keyFile = new File("/certs/server.key");
        if (certFile.exists() && keyFile.exists()) {
            SslContext sslContext = GrpcSslContexts.forServer(certFile, keyFile).build();
            Server tlsServer = NettyServerBuilder.forPort(8443)
                    .sslContext(sslContext)
                    .addService(new BenchmarkServiceImpl())
                    .build()
                    .start();
        }
        System.out.println("Application started.");
    }
}
