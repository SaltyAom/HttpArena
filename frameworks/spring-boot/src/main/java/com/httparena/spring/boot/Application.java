package com.httparena.spring.boot;

import com.zaxxer.hikari.HikariConfig;
import com.zaxxer.hikari.HikariDataSource;
import org.postgresql.ds.PGSimpleDataSource;
import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.context.annotation.Bean;
import org.springframework.jdbc.core.simple.JdbcClient;
import org.sqlite.SQLiteDataSource;

import javax.sql.DataSource;
import java.net.URI;
import java.net.URISyntaxException;

@SpringBootApplication
@EnableConfigurationProperties(HttpArenaProperties.class)
public class Application {

	public static void main(String[] args) {
		SpringApplication.run(Application.class, args);
	}

	@Qualifier("sqlite")
	@Bean
	public DataSource sqliteDataSource() {
		SQLiteDataSource sqLiteDataSource = new SQLiteDataSource();
		sqLiteDataSource.setUrl("jdbc:sqlite:/data/benchmark.db");
		sqLiteDataSource.setReadOnly(true);
		return sqLiteDataSource;
	}

	@Bean
	@Qualifier("sqlite")
	public JdbcClient sqliteJdbcClient(@Qualifier("sqlite") DataSource dataSource) {
		return JdbcClient.create(dataSource);
	}

	@Bean
	@Qualifier("postgresql")
	public DataSource postgresqlDataSource(HttpArenaProperties httpArenaProperties) throws URISyntaxException {
		URI uri = new URI(httpArenaProperties.postgresUrl());
		String jdbcUrl = "jdbc:postgresql://" + uri.getHost() + ":" + uri.getPort() + uri.getPath();
		String[] userInfo = uri.getUserInfo().split(":");
		PGSimpleDataSource pgSimpleDataSource = new PGSimpleDataSource();
		pgSimpleDataSource.setUrl(jdbcUrl);
		pgSimpleDataSource.setUser(userInfo[0]);
		pgSimpleDataSource.setPassword(userInfo[1]);
		pgSimpleDataSource.setReadOnly(true);
		return pgSimpleDataSource;
	}

	@Bean
	@Qualifier("postgresql-pool")
	public DataSource postgresqlPoolDataSource(@Qualifier("postgresql") DataSource dataSource) {
		HikariConfig configuration = new HikariConfig();
		configuration.setDataSource(dataSource);
		return new HikariDataSource(configuration);
	}

	@Bean
	@Qualifier("postgresql")
	public JdbcClient postgresqlJdbcClient(@Qualifier("postgresql-pool") DataSource dataSource) {
		return JdbcClient.create(dataSource);
	}
}
