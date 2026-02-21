export interface OfferingTemplate {
	key: string;
	label: string;
	icon: string;
	description: string;
	offerName: string;
	offeringDescription: string;
	productType: string;
	monthlyPrice: number | null;
	visibility: 'public' | 'private';
}

export const OFFERING_TEMPLATES: OfferingTemplate[] = [
	{
		key: 'basic-vps',
		label: 'Basic Linux VPS',
		icon: '🖥️',
		description: 'General-purpose KVM virtual server',
		offerName: 'Basic Linux VPS',
		offeringDescription:
			'A reliable KVM-based virtual private server running Ubuntu 22.04. Perfect for web apps, APIs, and general workloads.',
		productType: 'compute',
		monthlyPrice: 2,
		visibility: 'public',
	},
	{
		key: 'web-server',
		label: 'Web Server',
		icon: '🌐',
		description: 'Pre-configured LEMP stack for web hosting',
		offerName: 'Managed Web Server',
		offeringDescription:
			'Pre-configured LEMP stack (Linux, Nginx, MySQL, PHP) ready for web hosting. Includes SSL and firewall setup.',
		productType: 'compute',
		monthlyPrice: 3,
		visibility: 'public',
	},
	{
		key: 'gpu-ml',
		label: 'GPU ML Instance',
		icon: '🤖',
		description: 'NVIDIA GPU server for ML training',
		offerName: 'GPU ML Instance',
		offeringDescription:
			'NVIDIA GPU-equipped server for machine learning training and inference. Includes CUDA toolkit and PyTorch.',
		productType: 'gpu',
		monthlyPrice: 50,
		visibility: 'public',
	},
	{
		key: 'database',
		label: 'Database Server',
		icon: '🗄️',
		description: 'Dedicated PostgreSQL database server',
		offerName: 'Managed Database Server',
		offeringDescription:
			'Dedicated database server running PostgreSQL 16. Includes automated backups via cron and tuned configuration.',
		productType: 'compute',
		monthlyPrice: 5,
		visibility: 'public',
	},
	{
		key: 'dev-box',
		label: 'Dev Environment',
		icon: '💻',
		description: 'Personal cloud development workspace',
		offerName: 'Cloud Dev Box',
		offeringDescription:
			'A personal development environment with common tools pre-installed: git, docker, node, python, and vim.',
		productType: 'compute',
		monthlyPrice: 4,
		visibility: 'public',
	},
	{
		key: 'minecraft',
		label: 'Minecraft Server',
		icon: '⛏️',
		description: 'Minecraft Java Edition game server',
		offerName: 'Minecraft Game Server',
		offeringDescription:
			'Minecraft Java Edition server with Paper engine. Supports 10-20 players. Includes auto-restart on crash.',
		productType: 'compute',
		monthlyPrice: 6,
		visibility: 'public',
	},
];
