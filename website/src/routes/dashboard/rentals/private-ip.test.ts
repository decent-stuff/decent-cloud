import { describe, it, expect } from 'vitest';

/** Returns true if the IP is RFC1918 private (10.x, 172.16-31.x, 192.168.x) */
function isPrivateIp(ip: string): boolean {
	const parts = ip.split('.').map(Number);
	if (parts.length !== 4) return false;
	if (parts[0] === 10) return true;
	if (parts[0] === 172 && parts[1] >= 16 && parts[1] <= 31) return true;
	if (parts[0] === 192 && parts[1] === 168) return true;
	return false;
}

describe('isPrivateIp', () => {
	it('detects 10.x.x.x as private', () => {
		expect(isPrivateIp('10.0.0.1')).toBe(true);
		expect(isPrivateIp('10.255.255.255')).toBe(true);
	});

	it('detects 172.16-31.x.x as private', () => {
		expect(isPrivateIp('172.16.0.146')).toBe(true);
		expect(isPrivateIp('172.31.255.255')).toBe(true);
	});

	it('does not flag 172.15.x.x or 172.32.x.x as private', () => {
		expect(isPrivateIp('172.15.0.1')).toBe(false);
		expect(isPrivateIp('172.32.0.1')).toBe(false);
	});

	it('detects 192.168.x.x as private', () => {
		expect(isPrivateIp('192.168.1.1')).toBe(true);
		expect(isPrivateIp('192.168.0.100')).toBe(true);
	});

	it('does not flag public IPs as private', () => {
		expect(isPrivateIp('8.8.8.8')).toBe(false);
		expect(isPrivateIp('203.0.113.1')).toBe(false);
		expect(isPrivateIp('1.2.3.4')).toBe(false);
	});

	it('handles invalid input gracefully', () => {
		expect(isPrivateIp('')).toBe(false);
		expect(isPrivateIp('not-an-ip')).toBe(false);
		expect(isPrivateIp('1.2.3')).toBe(false);
	});
});

/** Derives the connectable IP: prefers public_ip, falls back to non-private ip_address */
function connectableIp(details: Record<string, unknown> | null): string | null {
	if (!details) return null;
	if (typeof details.public_ip === 'string') return details.public_ip;
	if (typeof details.ip_address === 'string' && !isPrivateIp(details.ip_address)) return details.ip_address;
	return null;
}

describe('connectableIp derivation', () => {
	it('prefers public_ip over ip_address', () => {
		const details = { public_ip: '203.0.113.5', ip_address: '172.16.0.146' };
		expect(connectableIp(details)).toBe('203.0.113.5');
	});

	it('falls back to public ip_address when no public_ip', () => {
		const details = { ip_address: '203.0.113.1' };
		expect(connectableIp(details)).toBe('203.0.113.1');
	});

	it('returns null for private ip_address without public_ip', () => {
		const details = { ip_address: '172.16.0.146' };
		expect(connectableIp(details)).toBeNull();
	});

	it('returns null for null details', () => {
		expect(connectableIp(null)).toBeNull();
	});
});

describe('connection details display logic', () => {
	it('prefers gateway over direct IP', () => {
		const contract = {
			gateway_subdomain: 'k7m2p4.dc-lk.dev-gw.decent-cloud.org',
			gateway_ssh_port: 20000,
			provisioning_instance_details: JSON.stringify({ ip_address: '172.16.0.146' }),
		};
		const useGateway = !!(contract.gateway_subdomain && contract.gateway_ssh_port);
		expect(useGateway).toBe(true);
	});

	it('shows pending message for private IP without gateway or public_ip', () => {
		const contract = {
			gateway_subdomain: null,
			gateway_ssh_port: null,
			provisioning_instance_details: JSON.stringify({ ip_address: '172.16.0.146' }),
		};
		const useGateway = !!(contract.gateway_subdomain && contract.gateway_ssh_port);
		const details = JSON.parse(contract.provisioning_instance_details);
		const ip = connectableIp(details);
		expect(useGateway).toBe(false);
		expect(ip).toBeNull();
		// Only private ip_address remains → pending message
		expect(details.ip_address).toBeTruthy();
	});

	it('shows direct public_ip from cloud provisioning', () => {
		const contract = {
			gateway_subdomain: null,
			gateway_ssh_port: null,
			provisioning_instance_details: JSON.stringify({ public_ip: '49.12.34.56', ip_address: '172.16.0.146' }),
		};
		const useGateway = !!(contract.gateway_subdomain && contract.gateway_ssh_port);
		const details = JSON.parse(contract.provisioning_instance_details);
		const ip = connectableIp(details);
		expect(useGateway).toBe(false);
		expect(ip).toBe('49.12.34.56');
	});

	it('shows direct ip_address when it is public', () => {
		const contract = {
			gateway_subdomain: null,
			gateway_ssh_port: null,
			provisioning_instance_details: JSON.stringify({ ip_address: '203.0.113.1' }),
		};
		const useGateway = !!(contract.gateway_subdomain && contract.gateway_ssh_port);
		const details = JSON.parse(contract.provisioning_instance_details);
		const ip = connectableIp(details);
		expect(useGateway).toBe(false);
		expect(ip).toBe('203.0.113.1');
	});
});
