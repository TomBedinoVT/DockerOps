# Exemple complet de structure avec volumes

## Structure du repository

```
my-dockerops-repo/
├── stacks.yaml
├── volumes.yaml
├── nfs.yaml
├── config/
│   ├── nginx.conf
│   └── app.conf
├── static/
│   ├── index.html
│   └── style.css
├── logs/
└── web-stack/
    └── docker-compose.yml
```

## Fichiers de configuration

### stacks.yaml
```yaml
- name: web-stack
```

### volumes.yaml
```yaml
- id: "app_data"
  type: "volume"
  path: "app_data_volume"

- id: "config_files"
  type: "binding"
  path: "config"

- id: "static_files"
  type: "binding"
  path: "static"

- id: "logs"
  type: "binding"
  path: "logs"
```

### nfs.yaml
```yaml
path: "/mnt/nfs/dockerops"
```

### web-stack/docker-compose.yml
```yaml
version: '3.8'

services:
  web:
    image: nginx:alpine
    ports:
      - "80:80"
    volumes:
      - "config_files:/etc/nginx/conf.d"
      - "static_files:/usr/share/nginx/html"
      - "logs:/var/log/nginx"

  app:
    image: myapp:latest
    volumes:
      - "app_data:/app/data"
      - "config_files:/app/config"
      - "logs:/app/logs"
```

## Transformation automatique

### Avant le traitement
```yaml
volumes:
  - "config_files:/etc/nginx/conf.d"
  - "static_files:/usr/share/nginx/html"
```

### Après le traitement
```yaml
volumes:
  - "/mnt/nfs/dockerops/config_files:/etc/nginx/conf.d"
  - "/mnt/nfs/dockerops/static_files:/usr/share/nginx/html"
```

## Flux de traitement

1. **Lecture des configurations** : `volumes.yaml` et `nfs.yaml`
2. **Traitement des volumes** :
   - `app_data` (volume) → Création du volume Docker `app_data_volume`
   - `config_files` (binding) → Copie de `./config/` vers `/mnt/nfs/dockerops/config_files/`
   - `static_files` (binding) → Copie de `./static/` vers `/mnt/nfs/dockerops/static_files/`
   - `logs` (binding) → Copie de `./logs/` vers `/mnt/nfs/dockerops/logs/`
3. **Modification du docker-compose** : Remplacement des IDs par les chemins NFS
4. **Déploiement** : Stack déployé avec les volumes modifiés

## Résultat final

Le stack sera déployé avec :
- Volume Docker `app_data_volume` pour les données persistantes
- Montages NFS pour les fichiers de configuration, statiques et logs
- Compatibilité totale avec Docker Swarm 