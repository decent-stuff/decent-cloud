/** Derive SSH login username from the operating system name.
 * Ubuntu cloud images use `ubuntu`; most others default to `root`. */
export function sshUsername(operatingSystem?: string | null): string {
	if (!operatingSystem) return 'root';
	const os = operatingSystem.toLowerCase();
	if (os.includes('ubuntu')) return 'ubuntu';
	if (os.includes('debian')) return 'root';
	if (os.includes('fedora')) return 'fedora';
	if (os.includes('centos')) return 'centos';
	if (os.includes('alma')) return 'almalinux';
	if (os.includes('rocky')) return 'rocky';
	return 'root';
}

/** Returns true if the IP is RFC1918 private (10.x, 172.16-31.x, 192.168.x) */
export function isPrivateIp(ip: string): boolean {
	const parts = ip.split('.').map(Number);
	if (parts.length !== 4) return false;
	if (parts[0] === 10) return true;
	if (parts[0] === 172 && parts[1] >= 16 && parts[1] <= 31) return true;
	if (parts[0] === 192 && parts[1] === 168) return true;
	return false;
}

/** Derives the connectable IP: prefers public_ip, falls back to non-private ip_address */
export function connectableIp(details: Record<string, unknown> | null): string | null {
	if (!details) return null;
	if (typeof details.public_ip === 'string') return details.public_ip;
	if (typeof details.ip_address === 'string' && !isPrivateIp(details.ip_address))
		return details.ip_address;
	return null;
}
