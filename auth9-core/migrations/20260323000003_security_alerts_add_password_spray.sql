-- Add password_spray and slow_brute_force to security_alerts alert_type enum
ALTER TABLE security_alerts MODIFY COLUMN alert_type ENUM(
    'brute_force',
    'slow_brute_force',
    'password_spray',
    'new_device',
    'impossible_travel',
    'suspicious_ip'
) NOT NULL;
