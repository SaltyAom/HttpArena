FROM mcr.microsoft.com/dotnet/sdk:10.0-preview AS build
WORKDIR /app
COPY . .
RUN dotnet publish -c Release -o out

FROM mcr.microsoft.com/dotnet/aspnet:10.0-preview-alpine
WORKDIR /app
COPY --from=build /app/out .
EXPOSE 8080 8443
ENTRYPOINT ["dotnet", "aspnet-grpc.dll"]
