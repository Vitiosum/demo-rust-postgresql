-- Jeu de données de la démo vidéo : 9 incidents, dates relatives à l'instant du rechargement.
-- Résultat attendu : 3 open, 2 investigating, 4 resolved ; 3 marqués « SLA dépassé » avec SLA_CRITICAL_MINUTES=30.
BEGIN;
TRUNCATE incidents;
INSERT INTO incidents (title, service, severity, status, description, created_at, updated_at) VALUES
('Paiements refusés par le PSP en 502', 'checkout', 'critical', 'investigating', E'Depuis 14 h 02, environ 30 % des paiements échouent avec une 502 côté PSP.\nPremière piste : rotation du certificat client.', now() - interval '35 min', now() - interval '6 min'),
('Latence p95 > 2 s sur la recherche', 'search-api', 'high', 'open', 'Le p95 est passé de 180 ms à 2,3 s après le déploiement de 13 h 40.', now() - interval '42 min', now() - interval '42 min'),
('File de mails en retard', 'notifications', 'medium', 'open', NULL, now() - interval '1 hour 20 min', now() - interval '1 hour 20 min'),
('Erreurs 401 intermittentes à la connexion', 'auth', 'high', 'investigating', 'Tokens rejetés juste après émission sur une instance sur deux.', now() - interval '3 hours', now() - interval '2 hours'),
('Export CSV tronqué au-delà de 10 000 lignes', 'reporting', 'medium', 'resolved', NULL, now() - interval '1 day', now() - interval '20 hours'),
('Faute de frappe sur la page tarifs', 'website', 'low', 'resolved', NULL, now() - interval '2 days', now() - interval '2 days'),
('Disque des logs à 92 %', 'observability', 'medium', 'resolved', NULL, now() - interval '3 days', now() - interval '3 days'),
('Webhooks GitHub en double', 'ci', 'low', 'open', NULL, now() - interval '5 hours', now() - interval '5 hours'),
('Base de cache saturée après la campagne', 'catalog', 'critical', 'resolved', 'Montée de gamme Redis appliquée, trafic rétabli.', now() - interval '4 days', now() - interval '4 days');
COMMIT;
