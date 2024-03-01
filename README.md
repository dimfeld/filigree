# filigree

Filigree is a web app framework based around Rust's Axum library. Its main feature is a templating system that generates data models and other functionality from configuration files, to reduce the amount of boilerplate needed to build your app.

This project is in active development, with things basically working but many features to come.

## Features

- Configure your application using a set of easy-to-read TOML files
- A server implementation with all the basics, generated for you but fully customizable
- Confidently edit any generated file. Filigree will do a 3-way merge to integrate future changes
- Email templates and email sending service integration
- Data Models
  - Declarative configuration of model fields
  - Generate migrations both when creating a new model, and when changing it later
  - SQL queries to do all the basics
  - A full set of CRUD endpoints for each model
  - Tests for all your endpoints
  - Parent-child model relationships
- Authentication
  - Traditional username/password
  - Passwordless login via email
  - OAuth2 Login
- Permissions system
 
And more to come!

[Roadmap](https://imfeld.dev/notes/projects_filigree)
