# DockerOps CLI

Un outil CLI simple en Rust pour gérer les stacks Docker Swarm depuis des répertoires GitHub.

## Fonctionnalités

- **Watch** : Clone un répertoire GitHub, lit le fichier `stacks.yaml`, et déploie les stacks Docker Swarm
- **Reconcile** : Affiche l'état actuel des stacks et images dans la base de données
- **Stop** : Arrête l'application et supprime toutes les stacks et images
- **Version** : Affiche les informations de version

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

### Table `repository_cache`
- `id` : Identifiant unique (INTEGER PRIMARY KEY)
- `url` : URL du répertoire GitHub (TEXT UNIQUE)
- `last_watch` : Timestamp du dernier watch (TEXT)

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
1. Vérifier que le répertoire n'est pas déjà en cache
2. Cloner le répertoire GitHub complet
3. Réinitialiser les compteurs de références d'images
4. Lire le fichier `stacks.yaml` pour obtenir la liste des stacks
5. Pour chaque stack, chercher le dossier correspondant
6. Trouver le fichier docker-compose dans chaque dossier de stack
7. Calculer le hash MD5 du docker-compose
8. Extraire les images Docker des fichiers YAML
9. Traiter les images :
   - Vérifier les SHA via l'API Docker Hub
   - Supprimer les images avec compteur à 0
   - Pull les images mises à jour ou manquantes
10. Déployer la stack avec `docker stack deploy`
11. Stocker les informations de la stack dans la base de données
12. Ajouter le répertoire au cache
13. Nettoyer le répertoire cloné temporaire

### Reconcile - Afficher l'état de la base de données

```bash
./dockerops reconcile
```

Cette commande affiche :
- Les répertoires en cache avec leur dernier watch
- Toutes les stacks stockées avec leur statut et hash
- Toutes les images stockées avec leur nombre de références

**Note** : Cette commande nécessite qu'au moins un répertoire ait été ajouté avec `watch`.

### Stop - Arrêter l'application et nettoyer

```bash
./dockerops stop
```

Cette commande va :
1. Supprimer toutes les stacks Docker Swarm
2. Supprimer toutes les images Docker
3. Nettoyer la base de données
4. Supprimer le cache des répertoires
5. Arrêter l'application

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

# Afficher la version
./dockerops version

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
