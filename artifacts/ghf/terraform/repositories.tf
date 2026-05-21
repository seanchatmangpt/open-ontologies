# GitHub Repositories





resource "github_repository" "open_ontologies" {
  name        = "open-ontologies"
  description = "AI-native ontology engine CLI"
  visibility  = "public"

  has_issues = true
  has_wiki   = false
  auto_init  = true
}

