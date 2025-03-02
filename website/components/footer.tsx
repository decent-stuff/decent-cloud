import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faGithub, faXTwitter, faDiscord } from '@fortawesome/free-brands-svg-icons';
import { faDatabase, faMessage, faComments } from '@fortawesome/free-solid-svg-icons';

const Footer = () => {
    return (
        <footer className="text-center mt-30 py-8 bg-gradient-to-b from-transparent to-black text-white text-sm">
            <div className="container mx-auto px-6">
                {/* Analytics Section */}
                <div className="mb-8 grid grid-cols-1 md:grid-cols-3 gap-8">
                    <div>
                        <h3 className="text-white text-lg font-semibold mb-4">About</h3>
                        <p className="text-white/70 text-sm mb-2">
                            Decent Cloud is a decentralized cloud resource marketplace with trustworthy reputations.
                        </p>
                    </div>

                    <div>
                        <h3 className="text-white text-lg font-semibold mb-4">Analytics</h3>
                        <ul className="space-y-2">
                            <li>
                                <a
                                    href="/ledger"
                                    className="text-white/70 hover:text-blue-400 transition duration-300 text-sm flex items-center gap-2"
                                >
                                    <FontAwesomeIcon icon={faDatabase} className="h-4 w-4"/>
                                    <span>Ledger Explorer</span>
                                </a>
                            </li>
                            <li>
                                <a
                                    href="#"
                                    className="text-white/50 text-sm flex items-center gap-2 cursor-not-allowed"
                                >
                                    <span>User and Provider Reputation (Coming Soon)</span>
                                </a>
                            </li>
                        </ul>
                    </div>

                    <div>
                        <h3 className="text-white text-lg font-semibold mb-4">Connect</h3>
                        <ul className="space-y-2">
                            <li>
                                <a
                                    href="https://oc.app/community/fn5ke-pyaaa-aaaac-aishq-cai"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-white/70 hover:text-blue-400 transition duration-300 text-sm flex items-center gap-2"
                                >
                                    <FontAwesomeIcon icon={faMessage} className="h-4 w-4"/>
                                    <span>OpenChat Community</span>
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://github.com/orgs/decent-stuff/discussions"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-white/70 hover:text-blue-400 transition duration-300 text-sm flex items-center gap-2"
                                >
                                    <FontAwesomeIcon icon={faComments} className="h-4 w-4"/>
                                    <span>GitHub Discussions</span>
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://x.com/DecentCloud_org"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-white/70 hover:text-blue-400 transition duration-300 text-sm flex items-center gap-2"
                                >
                                    <FontAwesomeIcon icon={faXTwitter} className="h-4 w-4"/>
                                    <span>Twitter</span>
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://discord.gg/ypHQaX6u3F"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-white/70 hover:text-blue-400 transition duration-300 text-sm flex items-center gap-2"
                                >
                                    <FontAwesomeIcon icon={faDiscord} className="h-4 w-4"/>
                                    <span>Discord</span>
                                </a>
                            </li>
                            <li>
                                <a
                                    href="https://github.com/decent-stuff"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-white/70 hover:text-blue-400 transition duration-300 text-sm flex items-center gap-2"
                                    aria-label="GitHub"
                                >
                                    <FontAwesomeIcon icon={faGithub} className="h-4 w-4"/>
                                    <span>GitHub</span>
                                </a>
                            </li>
                        </ul>
                    </div>
                </div>

                {/* Copyright */}
                <div className="flex flex-col md:flex-row items-center justify-between py-4 border-t border-white/10 space-y-3 md:space-y-0">
                    <div className="text-white/80 text-sm">
                        © {new Date().getFullYear()} Decent Cloud. All rights reserved.
                    </div>
                    <div className="text-white/60 text-xs">
                        Built with ❤️ by Decent Cloud Team
                    </div>
                </div>
            </div>
        </footer>
    );
};

export default Footer;
