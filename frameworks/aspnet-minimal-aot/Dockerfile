FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends --no-install-suggests clang zlib1g-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN dotnet publish -c Release -o out /p:OptimizationPreference=Speed /p:GarbageCollectionAdaptationMode=0 /p:IlcInstructionSet=native

FROM mcr.microsoft.com/dotnet/aspnet:10.0


ADD https://packages.microsoft.com/config/debian/12/packages-microsoft-prod.deb /packages-microsoft-prod.deb

RUN dpkg -i packages-microsoft-prod.deb && rm packages-microsoft-prod.deb \
    && apt-get update \
    && apt-get install -y --no-install-recommends libmsquic \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app/out .
EXPOSE 8080 8443/tcp 8443/udp

ENTRYPOINT ["./aspnet-minimal-aot"]