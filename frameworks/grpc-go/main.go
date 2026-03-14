package main

import (
	"context"
	"crypto/tls"
	"fmt"
	"net"
	"os"
	"runtime"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials"

	pb "grpc-go/proto"
)

type server struct {
	pb.UnimplementedBenchmarkServiceServer
}

func (s *server) GetSum(_ context.Context, req *pb.SumRequest) (*pb.SumReply, error) {
	return &pb.SumReply{Result: req.A + req.B}, nil
}

func main() {
	runtime.GOMAXPROCS(runtime.NumCPU())

	// h2c on 8080
	lis, err := net.Listen("tcp", ":8080")
	if err != nil {
		panic(err)
	}
	s := grpc.NewServer()
	pb.RegisterBenchmarkServiceServer(s, &server{})

	go func() {
		if err := s.Serve(lis); err != nil {
			panic(err)
		}
	}()

	// TLS on 8443 if certs exist
	certFile := "/certs/server.crt"
	keyFile := "/certs/server.key"
	if _, err := os.Stat(certFile); err == nil {
		cert, err := tls.LoadX509KeyPair(certFile, keyFile)
		if err != nil {
			fmt.Printf("Failed to load TLS certs: %v\n", err)
		} else {
			tlsLis, err := net.Listen("tcp", ":8443")
			if err != nil {
				panic(err)
			}
			creds := credentials.NewTLS(&tls.Config{Certificates: []tls.Certificate{cert}})
			tlsServer := grpc.NewServer(grpc.Creds(creds))
			pb.RegisterBenchmarkServiceServer(tlsServer, &server{})
			go func() {
				if err := tlsServer.Serve(tlsLis); err != nil {
					panic(err)
				}
			}()
		}
	}

	fmt.Println("Application started.")
	select {}
}
