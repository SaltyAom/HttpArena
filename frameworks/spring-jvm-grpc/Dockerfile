FROM eclipse-temurin:21-jdk AS build
WORKDIR /app

# Install Gradle
ENV GRADLE_VERSION=8.12
RUN apt-get update && apt-get install -y --no-install-recommends wget unzip && \
    wget -q "https://services.gradle.org/distributions/gradle-${GRADLE_VERSION}-bin.zip" -O gradle.zip && \
    unzip -q gradle.zip && \
    rm gradle.zip && \
    apt-get purge -y wget unzip && apt-get autoremove -y && rm -rf /var/lib/apt/lists/*
ENV PATH="/app/gradle-${GRADLE_VERSION}/bin:${PATH}"

COPY build.gradle settings.gradle ./
COPY src ./src

RUN gradle bootJar --no-daemon -q

FROM eclipse-temurin:21-jre
WORKDIR /app
COPY --from=build /app/build/libs/spring-jvm-grpc-1.0.0.jar app.jar
EXPOSE 8080 8443
ENTRYPOINT ["java", "-jar", "app.jar"]
