# Gestion des Volumes dans DockerOps

## Vue d'ensemble

DockerOps gère les volumes de manière centralisée pour résoudre les problèmes de bindings avec Docker Swarm. Le système utilise deux types de volumes :

1. **Volumes Docker** : Volumes gérés par Docker
2. **Bindings** : Fichiers/dossiers copiés vers un serveur NFS

## Structure des fichiers

### volumes.yaml

Définit tous les volumes utilisés par les stacks :

```yaml
- id: "app_data"
  type: "volume"
  path: "app_data_volume"

- id: "config_files"
  type: "binding"
  path: "config"
```

- `id` : Identifiant unique du volume (utilisé dans docker-compose.yml)
- `type` : `volume` ou `binding`
- `path` : 
  - Pour `volume` : nom du volume Docker
  - Pour `binding` : chemin relatif dans le repository

### nfs.yaml

Configuration du serveur NFS pour les bindings :

```yaml
path: "/mnt/nfs/dockerops"
```

## Utilisation dans docker-compose.yml

### Format des volumes

Les volumes sont définis directement dans les services, pas dans une section `volumes` séparée :

```yaml
services:
  myapp:
    image: myapp:latest
    volumes:
      - "volume_id:container_path"
```

### Exemple

```yaml
services:
  web:
    image: nginx:alpine
    volumes:
      - "config_files:/etc/nginx/conf.d"
      - "static_files:/usr/share/nginx/html"
```

**Note importante** : Il n'y a pas de section `volumes` dans le docker-compose.yml. Les IDs de volumes sont référencés directement dans les services.

## Fonctionnement

### 1. Traitement des volumes

Lors du `reconcile` ou `watch`, DockerOps :

1. Lit `volumes.yaml` et `nfs.yaml`
2. Pour chaque volume de type `volume` :
   - Crée le volume Docker s'il n'existe pas
3. Pour chaque volume de type `binding` :
   - Copie le contenu du repository vers NFS
   - Supprime l'ancien contenu sur NFS s'il existe

### 2. Modification du docker-compose.yml

DockerOps modifie automatiquement le docker-compose.yml :

- **Volumes Docker** : Restent inchangés
- **Bindings** : Sont remplacés par le chemin NFS

Exemple de transformation :
```yaml
# Avant (dans le repository)
- "config_files:/etc/nginx/conf.d"

# Après (déployé)
- "/mnt/nfs/dockerops/config_files:/etc/nginx/conf.d"
```

## Structure de répertoires

```
repository/
├── stacks.yaml
├── volumes.yaml
├── nfs.yaml
├── config/           # Binding volume
│   ├── nginx.conf
│   └── app.conf
├── static/           # Binding volume
│   ├── index.html
│   └── style.css
├── logs/             # Binding volume
└── my-stack/
    └── docker-compose.yml
```

## Avantages

1. **Compatibilité Swarm** : Les bindings sont convertis en montages NFS
2. **Centralisation** : Tous les volumes définis dans un seul fichier
3. **Flexibilité** : Support des volumes Docker et des bindings
4. **Automatisation** : Copie automatique vers NFS à chaque reconcile

## Configuration requise

1. **Serveur NFS** : Doit être accessible depuis tous les nœuds Swarm
2. **Permissions** : DockerOps doit avoir les droits d'écriture sur NFS
3. **Montage NFS** : Le chemin NFS doit être monté sur tous les nœuds

## Dépannage

### Erreur de copie NFS
- Vérifiez que le serveur NFS est accessible
- Vérifiez les permissions sur le répertoire NFS
- Vérifiez que le chemin local existe dans le repository

### Volume Docker non créé
- Vérifiez que Docker est accessible
- Vérifiez les permissions Docker

### Binding non transformé
- Vérifiez que l'ID dans docker-compose.yml correspond à volumes.yaml
- Vérifiez que le fichier volumes.yaml est présent 