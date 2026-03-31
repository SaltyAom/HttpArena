FROM mcr.microsoft.com/dotnet/sdk:10.0-preview-alpine AS build
WORKDIR /app
COPY . .
RUN dotnet publish -c Release -o out

FROM mcr.microsoft.com/dotnet/aspnet:10.0-preview-alpine
RUN apk add --no-cache libmsquic
WORKDIR /app
COPY --from=build /app/out .
EXPOSE 8080 8443/tcp 8443/udp
ENTRYPOINT ["dotnet", "aspnet-mvc.dll"]
