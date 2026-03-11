FROM mcr.microsoft.com/dotnet/sdk:10.0-preview-alpine AS build
WORKDIR /source
COPY genhttp.csproj .
RUN dotnet restore -r linux-musl-x64
COPY Program.cs .
RUN dotnet publish -c Release -r linux-musl-x64 --self-contained -o /app

FROM mcr.microsoft.com/dotnet/runtime-deps:10.0-preview-alpine
WORKDIR /app
COPY --from=build /app .
RUN apk add --no-cache libmsquic
EXPOSE 8080 8443/tcp 8443/udp
ENTRYPOINT ["./genhttp"]
