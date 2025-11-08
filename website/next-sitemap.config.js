/** @type {import('next-sitemap').IConfig} */
module.exports = {
    siteUrl: process.env.SITE_URL || 'https://decent-cloud.org',
    generateRobotsTxt: true,
    outDir: './out',
}
