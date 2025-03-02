import InfoCard from "@/components/ui/info-card";
import HeaderSection from "@/components/ui/header";

const infoSections = [
    {
        title: "What is Decent Cloud?",
        icon: "🤔",
        content: `Think of us as the &quot;Airbnb of cloud services&quot; - just more fair and open! We&apos;re a community-driven platform
    that&apos;s shaking up the cloud oligopoly by enabling peer-to-peer resource sharing. Say goodbye to steep pricing and
    those pesky region-wide outages!<br/><br/>
    <strong>Key highlights:</strong><br/>
    • Provider reputations and reviews tracked in tamper-proof ledger<br/>
    • No gatekeepers or central control, completely permissionless<br/>
    • Self-sustaining with minimal fees<br/>
    • Community-driven evolution`
    },
    {
        title: "How Reputation Works",
        icon: "⭐",
        content: `Our reputation system builds trust through real-world transactions:<br/><br/>
    <strong>Earning Reputation:</strong><br/>
    • Reputation is earned through successful transactions between developers and node providers<br/>
    • When a developer pays a provider, both parties gain a small reputation boost<br/>
    • All reputation changes are recorded transparently on the blockchain<br/><br/>
    <strong>Why It Matters:</strong><br/>
    • Reputation is a valuable asset that takes time to build<br/>
    • Disputes cost reputation for both parties, discouraging frivolous claims<br/>
    • The system promotes high standards of conduct and service quality<br/>
    • Blockchain ensures tamper-resistance and transparent verification`
    },
    {
        title: "Become a validator!",
        icon: "⛏️",
        content: `Validators play a crucial role in ensuring ledger integrity.<br/>Anyone can be a validator! Here’s how you can get started:<br/><br/>
    1. Acquire DCT tokens (from peers or a DEX such as Kong)<br/>
    2. Run <code>dc np check-in</code> with your identity<br/>
    3. Get rewarded with block rewards<br/><br/>
    <strong>Quick facts:</strong><br/>
    • 50 DCT initial block reward<br/>
    • Unclaimed rewards are carried over to the next block<br/>
    • New block every 10 minutes<br/>
    • Reward halves every 210,000 blocks<br/>
    • Total supply: ~21M DCT`
    },
    {
        title: "What makes it safe?",
        icon: "💰",
        content: `It's actually safer than other marketplaces. Rather than relying on a behind-the-scenes database, all financial transactions are recorded on the blockchain, so there is no possibility of a fraud or a rug pull. Fees are paid in Decentralized Cloud Token (DCT), and conversion is automatic. What drives the value of DCT:<br/><br/>
    • <strong>Demand:</strong> All fees are paid in DCT. In addition, DCT can be used to pay for renting services or resources<br/>
    • <strong>Supply:</strong> DC tokens can be acquired by renting services or resources, or by mining/validating<br/>
    • <strong>Governance:</strong> Platform is permissionless and DAO-controlled, so all decisions are made by the community<br/>
    • <strong>Open Source:</strong> Anyone can contribute to the project, or fork it and start their own Decent Cloud.
    `
    },
    {
        title: "Whitepaper: Deep Dive into Decent Cloud",
        icon: "📑",
        content: `For those who prefer their tech docs well-done.<br/><br/>
    Our comprehensive whitepaper details the technical architecture, tokenomics, and governance model that makes Decent Cloud truly decent.<br/><br/>
    <a href="https://decent-stuff.github.io/website/decent-cloud-whitepaper.pdf" target="_blank" rel="noopener noreferrer" className="text-blue-500 hover:text-blue-600 transition-colors">Download the Whitepaper →</a>`
    }
];

const InfoSection = () => {
    return (
        <section id="info" className="pt-20">
            <div className="container mx-auto px-6 text-center">
                <HeaderSection title={"Want to know more?"} subtitle={"Find out more about how Decent Cloud works and its unique features."}/>

                <div className="grid grid-cols-1 gap-4 max-w-3xl mx-auto">
                    {infoSections.map((section, index) => (
                        <InfoCard key={index} {...section} />
                    ))}
                </div>
            </div>
        </section>
    );
};

export default InfoSection;
