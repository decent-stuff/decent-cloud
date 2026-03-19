import { describe, it, expect } from 'vitest';
import { isPrivateIp, connectableIp, sshUsername } from './network';

describe('isPrivateIp', () => {
	it('detects RFC1918 ranges', () => {
		expect(isPrivateIp('10.0.0.1')).toBe(true);
		expect(isPrivateIp('172.16.0.1')).toBe(true);
		expect(isPrivateIp('192.168.1.1')).toBe(true);
	});

	it('rejects public IPs', () => {
		expect(isPrivateIp('8.8.8.8')).toBe(false);
		expect(isPrivateIp('203.0.113.1')).toBe(false);
	});

	it('rejects invalid input', () => {
		expect(isPrivateIp('')).toBe(false);
		expect(isPrivateIp('not-an-ip')).toBe(false);
	});
});

describe('connectableIp', () => {
	it('prefers public_ip', () => {
		expect(connectableIp({ public_ip: '1.2.3.4', ip_address: '10.0.0.1' })).toBe('1.2.3.4');
	});

	it('uses public ip_address', () => {
		expect(connectableIp({ ip_address: '1.2.3.4' })).toBe('1.2.3.4');
	});

	it('rejects private ip_address', () => {
		expect(connectableIp({ ip_address: '10.0.0.1' })).toBeNull();
	});

	it('returns null for null', () => {
		expect(connectableIp(null)).toBeNull();
	});
});

describe('sshUsername', () => {
	it('returns ubuntu for Ubuntu OS', () => {
		expect(sshUsername('Ubuntu 22.04')).toBe('ubuntu');
		expect(sshUsername('ubuntu-24.04-lts')).toBe('ubuntu');
	});

	it('returns fedora for Fedora OS', () => {
		expect(sshUsername('Fedora 39')).toBe('fedora');
	});

	it('returns centos for CentOS', () => {
		expect(sshUsername('CentOS Stream 9')).toBe('centos');
	});

	it('returns almalinux for AlmaLinux', () => {
		expect(sshUsername('AlmaLinux 9')).toBe('almalinux');
	});

	it('returns rocky for Rocky Linux', () => {
		expect(sshUsername('Rocky Linux 9')).toBe('rocky');
	});

	it('returns root for Debian', () => {
		expect(sshUsername('Debian 12')).toBe('root');
	});

	it('returns root for null/undefined', () => {
		expect(sshUsername(null)).toBe('root');
		expect(sshUsername(undefined)).toBe('root');
	});

	it('returns root for unknown OS', () => {
		expect(sshUsername('SomeUnknownOS')).toBe('root');
	});
});
