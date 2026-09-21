/** One independent set of everything the app stores: its own portfolio, settings and layouts. */
export interface Profile {
  id: string;
  /** The user's own words. */
  name: string;
  created_at: string;
  /** Whether it has a password. */
  protected: boolean;
}

export interface ProfileList {
  profiles: Profile[];
  /** Id of the profile the app has open. */
  open: string;
  /** The open profile has a password and has not been unlocked yet. */
  locked: boolean;
  /** This device opens the open profile without asking for its password. */
  remembered: boolean;
}
