# DockerOps CLI

Un outil CLI simple en Rust pour gérer les stacks Docker Swarm depuis des répertoires GitHub.

## Fonctionnalités

- **Watch** : Clone un répertoire GitHub, lit le fichier `stacks.yaml`, et déploie les stacks Docker Swarm
- **Reconcile** : Affiche l'état actuel des stacks et images dans la base de données
- **Stop** : Arrête l'application et supprime toutes les stacks et images

## Structure de la base de données

### Table `images`
- `id` : Identifiant unique (INTEGER PRIMARY KEY)
- `name` : Nom de l'image Docker (TEXT UNIQUE)
- `reference_count` : Nombre de références à cette image (INTEGER)

### Table `stacks`
- `id` : Identifiant unique (INTEGER PRIMARY KEY)
- `name` : Nom de la stack (TEXT)
- `repository_url` : URL du répertoire GitHub (TEXT)
- `compose_path` : Chemin vers le fichier docker-compose (TEXT)
- `hash` : Hash MD5 du contenu du docker-compose (TEXT)
- `status` : Statut de la stack ("deployed", "stopped", "error") (TEXT)
- `UNIQUE(name, repository_url)` : Contrainte d'unicité

## Installation

```bash
# Cloner le projet
git clone <repository-url>
cd dockerops

# Compiler le projet
cargo build --release

# L'exécutable sera disponible dans target/release/dockerops
```

## Utilisation

### Surveiller un répertoire GitHub

```bash
./dockerops watch "https://github.com/user/repo"
```

Cette commande va :
1. Cloner le répertoire GitHub complet
2. Réinitialiser les compteurs de références d'images
3. Lire le fichier `stacks.yaml` pour obtenir la liste des stacks
4. Pour chaque stack, chercher le dossier correspondant
5. Trouver le fichier docker-compose dans chaque dossier de stack
6. Calculer le hash MD5 du docker-compose
7. Extraire les images Docker des fichiers YAML
8. Traiter les images :
   - Vérifier les SHA via l'API Docker Hub
   - Supprimer les images avec compteur à 0
   - Pull les images mises à jour ou manquantes
9. Déployer la stack avec `docker stack deploy`
10. Stocker les informations de la stack dans la base de données
11. Nettoyer le répertoire cloné temporaire

### Reconcile - Afficher l'état de la base de données

```bash
./dockerops reconcile
```

Cette commande affiche :
- Toutes les stacks stockées avec leur statut et hash
- Toutes les images stockées avec leur nombre de références

### Stop - Arrêter l'application et nettoyer

```bash
./dockerops stop
```

Cette commande va :
1. Supprimer toutes les stacks Docker Swarm
2. Supprimer toutes les images Docker
3. Nettoyer la base de données
4. Arrêter l'application

## Structure du répertoire attendu

Le répertoire GitHub doit contenir :

```
repository/
├── stacks.yaml          # Définition des stacks
├── stack1/              # Dossier de la première stack
│   └── docker-compose.yml
├── stack2/              # Dossier de la deuxième stack
│   └── docker-compose.yml
└── ...
```

### Format du fichier stacks.yaml

```yaml
- name: Nom De la Stack
- name: Autre Stack
```

## Exemple d'utilisation

```bash
# Surveiller un répertoire GitHub
./dockerops watch "https://github.com/example/docker-swarm-stacks"

# Vérifier l'état de la base de données
./dockerops reconcile

# Arrêter l'application et nettoyer toutes les ressources
./dockerops stop
```

## Dépendances

- **clap** : Parsing des arguments CLI
- **tokio** : Runtime asynchrone
- **sqlx** : ORM pour SQLite
- **git2** : Clonage de répertoires Git
- **walkdir** : Parcours récursif des répertoires
- **md5** : Calcul des hashes MD5
- **serde** : Sérialisation/désérialisation
- **serde_yaml** : Parsing des fichiers YAML
- **reqwest** : Client HTTP pour l'API Docker Hub
- **anyhow** : Gestion d'erreurs

## Gestion des images Docker

L'application gère automatiquement les images Docker :

### Vérification des SHA
- Compare les SHA locaux avec ceux du registre Docker Hub
- Utilise l'API Docker Hub : `HEAD /v2/{repository}/manifests/{tag}`
- Détecte les images obsolètes et les met à jour

### Nettoyage automatique
- Réinitialise les compteurs de références à chaque watch/reconcile
- Supprime les images avec compteur à 0 (non utilisées)
- Nettoie la base de données des images supprimées

### Pull automatique
- Pull les images manquantes localement
- Pull les images mises à jour (SHA différent)
- Supprime l'ancienne version avant de pull la nouvelle

## Base de données

L'application utilise SQLite avec le fichier `dockerops.db` créé automatiquement dans le répertoire courant.

## Développement

```bash
# Installer les dépendances
cargo build

# Lancer les tests
cargo test

# Exécuter en mode debug
cargo run -- watch "https://github.com/example/repo"
```
